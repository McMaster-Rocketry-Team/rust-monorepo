use defmt::{info, unwrap};
use futures::future::join;
use lora_phy::mod_params::{Bandwidth, CodingRate, SpreadingFactor};

use crate::driver::timer::DelayUsWrapper;
use crate::utils::run_with_timeout;
use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
};
use heapless::String;

#[inline(never)]
pub async fn ground_test_avionics(device_manager: device_manager_type!()) -> ! {
    let timer = device_manager.timer;
    claim_devices!(
        device_manager,
        lora,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        status_indicator
    );
    let lora = lora.as_mut().unwrap();
    lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
    let mut rx_buffer = [0u8; 256];

    let indicator_fut = async {
        loop {
            status_indicator.set_enable(true).await;
            timer.sleep(50.0).await;
            status_indicator.set_enable(false).await;
            timer.sleep(2000.0).await;
        }
    };

    let avionics_fut = async {
        loop {
            let mut lora_message = String::<50>::new();
            match pyro1_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 1: Cont | ").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 1: No Cont | ").unwrap(),
                Err(_) => lora_message.push_str("Pyro 1: Error | ").unwrap(),
            };
            match pyro2_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 2: Cont").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 2: No Cont").unwrap(),
                Err(_) => lora_message.push_str("Pyro 2: Error").unwrap(),
            };

            info!("{}", lora_message.as_str());

            let modulation_params = lora
                .create_modulation_params(
                    SpreadingFactor::_12,
                    Bandwidth::_250KHz,
                    CodingRate::_4_8,
                    915_000_000,
                )
                .unwrap();
            let mut tx_params = lora
                .create_tx_packet_params(8, false, true, false, &modulation_params)
                .unwrap();
            lora.prepare_for_tx(&modulation_params, -9, true)
                .await
                .unwrap();
            lora.tx(
                &modulation_params,
                &mut tx_params,
                lora_message.as_bytes(),
                0xFFFFFF,
            )
            .await
            .unwrap();

            lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
            let rx_params = lora
                .create_rx_packet_params(8, false, 255, true, false, &modulation_params)
                .unwrap();
            lora.prepare_for_rx(
                &modulation_params,
                &rx_params,
                None,
                true,
                true,
                4,
                0x00FFFFFFu32,
            )
            .await
            .unwrap();

            match run_with_timeout(timer, 1000.0, lora.rx(&rx_params, &mut rx_buffer)).await {
                Ok(Ok((received_len, status))) => {
                    info!(
                        "Received {} bytes, snr: {}, rssi: {}",
                        received_len, status.snr, status.rssi
                    );
                    let rx_buffer = &rx_buffer[..(received_len as usize)];
                    if rx_buffer == b"VLF3 fire 1" {
                        info!("Firing pyro 1");
                        unwrap!(pyro1_ctrl.set_enable(true).await);
                        timer.sleep(1000.0).await;
                        unwrap!(pyro1_ctrl.set_enable(false).await);
                    } else if rx_buffer == b"VLF3 fire 2" {
                        info!("Firing pyro 2");
                        unwrap!(pyro2_ctrl.set_enable(true).await);
                        timer.sleep(1000.0).await;
                        unwrap!(pyro2_ctrl.set_enable(false).await);
                    }
                }
                Ok(Err(lora_error)) => {
                    info!("LoRa Error: {:?}", lora_error);
                }
                Err(_) => {
                    info!("RX Timeout");
                }
            }

            lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
            timer.sleep(2000.0).await;
        }
    };

    join(indicator_fut, avionics_fut).await;
    defmt::unreachable!()
}
