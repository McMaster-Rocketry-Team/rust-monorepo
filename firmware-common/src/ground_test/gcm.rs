use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator},
};
use defmt::{info, unwrap};
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    RxMode,
};

#[inline(never)]
pub async fn ground_test_gcm(device_manager: device_manager_type!()) -> ! {
    let mut delay = device_manager.delay;
    claim_devices!(device_manager, lora);

    let mut count = 0u8;

    let modulation_params = lora
        .create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            903_900_000,
        )
        .unwrap();
    let mut tx_params = lora
        .create_tx_packet_params(4, false, false, false, &modulation_params)
        .unwrap();
    let rx_pkt_params = lora
        .create_rx_packet_params(4, false, 50, false, false, &modulation_params)
        .unwrap();
    let mut receiving_buffer = [0u8; 222];
    loop {
        lora.prepare_for_rx(RxMode::Single(1000), &modulation_params, &rx_pkt_params)
            .await
            .unwrap();
        match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
            Ok((length, status)) => {
                let data = &receiving_buffer[0..(length as usize)];
                info!("Received {} bytes, rssi: {}, snr: {}", length, status.rssi, status.snr);
                if data.starts_with(b"Pyro 1: ") {
                    info!(
                        "{}",
                        core::str::from_utf8(data).unwrap()
                    );

                    count += 1;
                    info!("{}/3", count);
                    if count == 3 {
                        unwrap!(
                            lora.prepare_for_tx(
                                &modulation_params,
                                &mut tx_params,
                                9,
                                b"VLF4 fire 2"
                            )
                            .await
                        );
                        unwrap!(lora.tx().await);

                        info!("Sent fire message");
                        loop {
                            delay.delay_ms(1000).await;
                        }
                    }
                }
            }
            Err(err) => {
                info!("Error: {:?}", err);
            }
        }
    }
}
