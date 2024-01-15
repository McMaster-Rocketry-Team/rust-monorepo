use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
};
use defmt::{info, unwrap};
use futures::future::join;
use heapless::String;

#[inline(never)]
pub async fn ground_test_avionics(device_manager: device_manager_type!()) -> ! {
    let timer = device_manager.timer;
    claim_devices!(
        device_manager,
        vlp_phy,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        status_indicator
    );

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

            vlp_phy.tx(lora_message.as_bytes()).await;

            match vlp_phy.rx_with_timeout(1000).await {
                Ok(data) => {
                    info!("Received {} bytes", data.0.len);
                    let rx_buffer = data.1.as_slice();
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
                Err(lora_error) => {
                    info!("Radio Error: {:?}", lora_error);
                }
            }

            timer.sleep(2000.0).await;
        }
    };

    join(indicator_fut, avionics_fut).await;
    defmt::unreachable!()
}
