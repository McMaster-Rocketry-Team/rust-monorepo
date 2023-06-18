use defmt::{info, unwrap};
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    common::{device_mode::{write_device_mode, DeviceMode}, console::console_program::ConsoleProgram},
    driver::{serial::Serial, sys_reset::SysReset},
    try_or_warn,
};

pub struct ChangeMode<'a, F:Flash, C:Crc> {
    vlfs: &'a VLFS<F,C>,
}

impl<'a, F:Flash, C:Crc> ChangeMode<'a,F,C> {
    pub fn new(vlfs: &'a VLFS<F,C>) -> Self {
        Self {vlfs}
    }
}

impl<'a, F:Flash, C:Crc> ConsoleProgram for ChangeMode<'a,F,C> {
    fn id(&self) -> u64 {
        0x3
    }

    async fn run(&mut self, serial: &mut impl Serial, device_manager: device_manager_type!()) {
        let mut buffer = [0u8; 1];
        unwrap!(serial.read(&mut buffer).await);
        let new_device_mode = DeviceMode::try_from(buffer[0]).unwrap();
        info!("Changing device mode to {:?}", new_device_mode);
        try_or_warn!(write_device_mode(self.vlfs, new_device_mode).await);

        claim_devices!(device_manager, sys_reset);
        sys_reset.reset()
    }
}