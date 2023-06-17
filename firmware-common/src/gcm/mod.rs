use lora_phy::mod_traits::RadioKind;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
};

#[inline(never)]
pub async fn gcm_main(fs: &VLFS<impl Flash, impl Crc>, device_manager: device_manager_type!()) {
    let radio = device_manager.get_radio_application_layer().await;
}
