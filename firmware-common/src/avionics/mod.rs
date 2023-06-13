use lora_phy::mod_traits::RadioKind;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
};

pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;

pub async fn avionics_main(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) {
    let timer = device_manager.timer;
    claim_devices!(device_manager, buzzer);

    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;
}
