use defmt::warn;

use crate::common::device_manager::prelude::*;
use crate::device_manager_type;
use crate::{common::console::console_program::ConsoleProgram, create_rpc};

create_rpc! {
    enums {
        enum DeviceModel {
            VLF1,
            VLF2,
            VLF3,
            VLF4,
        }
    }
    rpc 0 WhoAmI {
        request()
        response(name: [u8; 64], model: DeviceModel, serial_number: [u8; 12])
    }
}

pub struct WhoAmI {}

impl WhoAmI {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConsoleProgram for WhoAmI {
    fn id(&self) -> u64 {
        0x0
    }

    async fn run(&mut self, serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        let result = run_rpc_server(
            serial,
            async || {
                // TODO
                let mut name = [0u8; 64];
                name[..5].copy_from_slice(b"VLF4\0");
                return WhoAmIResponse {
                    name: name,
                    model: DeviceModel::VLF4,
                    serial_number: [0u8; 12],
                };
            },
            async || {},
        )
        .await;
        if let Err(e) = result {
            warn!("rpc ended due to {:?}", e);
        }
    }
}
