use defmt::unwrap;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    common::device_mode::{write_device_mode, DeviceMode},
    device_manager_type,
    driver::{device_management::DeviceManagement, serial::Serial},
    try_or_warn,
};

pub struct ChangeMode {}

impl ChangeMode {
    pub fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x3
    }

    pub async fn start(
        &self,
        serial: &mut impl Serial,
        vlfs: &VLFS<impl Flash, impl Crc>,
        device_manager: device_manager_type!(),
    ) -> ! {
        let mut buffer = [0u8; 1];
        unwrap!(serial.read(&mut buffer).await);
        let new_device_mode = DeviceMode::try_from(buffer[0]).unwrap();
        try_or_warn!(write_device_mode(vlfs, new_device_mode).await);

        claim_devices!(device_manager, device_management);
        device_management.reset()
    }
}
