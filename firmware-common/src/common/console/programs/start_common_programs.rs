use futures::join;
use vlfs::VLFS;

use crate::common::console::console::Console;
use crate::common::console::console_program::start_console_program;
use crate::common::device_manager::prelude::*;
use crate::driver::serial::Serial;

use super::calibrate::Calibrate;
use super::change_mode::ChangeMode;

pub async fn start_common_programs<const N: usize>(
    device_manager: device_manager_type!(),
    console: &Console<impl Serial, N>,
    vlfs: &VLFS<impl Flash, impl Crc>,
) -> ! {
    let change_mode_fut = start_console_program(device_manager, console, ChangeMode::new(vlfs));
    let calibrate_fut = start_console_program(
        device_manager,
        console,
        Calibrate::new(vlfs, device_manager.delay),
    );

    #[allow(unreachable_code)]
    {
        join!(change_mode_fut, calibrate_fut);
    }
    defmt::unreachable!()
}
