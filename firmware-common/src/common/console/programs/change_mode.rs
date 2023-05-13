use defmt::unwrap;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    common::device_mode::{write_device_mode, DeviceMode},
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

    pub async fn start<T: Serial, F: Flash, C: Crc, D: DeviceManagement>(
        &self,
        serial: &mut T,
        vlfs: &VLFS<F, C>,
        device_mngmt: D,
    ) -> Result<(), ()>
    where
        F::Error: defmt::Format,
        F: defmt::Format,
    {
        let mut buffer = [0u8; 1];
        unwrap!(serial.read(&mut buffer).await);
        let new_device_mode = DeviceMode::try_from(buffer[0]).unwrap();
        try_or_warn!(write_device_mode(vlfs, new_device_mode).await);

        device_mngmt.reset();
    }
}
