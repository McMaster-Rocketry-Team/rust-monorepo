use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator},
};
use defmt::info;

#[inline(never)]
pub async fn ground_test_gcm(device_manager: device_manager_type!()) -> ! {
    let mut delay = device_manager.delay;
    claim_devices!(device_manager, radio_phy);

    let mut count = 0u8;

    loop {
        match radio_phy.rx().await {
            Ok(package) => {
                info!("Received {} bytes", package.0.len);
                let rx_buffer = package.1.as_slice();
                if rx_buffer.starts_with(b"Pyro 1: ") {
                    info!(
                        "Received continuity message: {}",
                        core::str::from_utf8(rx_buffer).unwrap()
                    );

                    count += 1;
                    info!("{}/3", count);
                    if count == 3 {
                        radio_phy.tx(b"VLF3 fire 1").await;

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
