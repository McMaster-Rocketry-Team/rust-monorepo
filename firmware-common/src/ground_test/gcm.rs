use core::str::FromStr;

use defmt::info;
use lora_phy::mod_params::{Bandwidth, CodingRate, SpreadingFactor};

use crate::driver::timer::DelayUsWrapper;
use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
};
use heapless::String;

#[inline(never)]
pub async fn ground_test_gcm(device_manager: device_manager_type!()) -> ! {
    let timer = device_manager.timer;
    claim_devices!(device_manager, lora);
    let lora = lora.as_mut().unwrap();
    lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
    let mut rx_buffer = [0u8; 256];

    let modulation_params = lora
        .create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            915_000_000,
        )
        .unwrap();
    let rx_params = lora
        .create_rx_packet_params(8, false, 255, true, false, &modulation_params)
        .unwrap();

    let mut count = 0u8;

    loop {
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

        match lora.rx(&rx_params, &mut rx_buffer).await {
            Ok((received_len, status)) => {
                info!(
                    "Received {} bytes, snr: {}, rssi: {}",
                    received_len, status.snr, status.rssi
                );
                let rx_buffer = &rx_buffer[..(received_len as usize)];
                if rx_buffer.starts_with(b"Pyro 1: ") {
                    info!(
                        "Received continuity message: {}",
                        core::str::from_utf8(rx_buffer).unwrap()
                    );
                    let mut tx_params = lora
                        .create_tx_packet_params(8, false, true, false, &modulation_params)
                        .unwrap();

                    count += 1;
                    info!("{}/3", count);
                    if count == 3 {
                        lora.prepare_for_tx(&modulation_params, -9, true)
                            .await
                            .unwrap();
                        lora.tx(&modulation_params, &mut tx_params, b"VLF3 fire 1", 0xFFFFFF)
                            .await
                            .unwrap();

                        info!("Sent fire message");
                        loop {
                            timer.sleep(1000.0).await;
                        }
                    }
                }
            }
            Err(err) => {
                info!("Error: {:?}", err);
            }
        }
    }
    defmt::unreachable!()
}
