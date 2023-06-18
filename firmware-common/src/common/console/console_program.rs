use crate::common::device_manager::prelude::*;
use crate::driver::serial::Serial;

use super::console::Console;

pub trait ConsoleProgram {
    fn id(&self) -> u64;
    async fn run(&mut self, serial: &mut impl Serial, device_manager: device_manager_type!());
}

pub async fn start_console_program<const N: usize>(
    device_manager: device_manager_type!(),
    console: &Console<impl Serial, N>,
    mut program: impl ConsoleProgram,
) -> ! {
    loop {
        let mut serial = console.wait_for_command(program.id()).await;
        program.run(&mut serial, device_manager).await;
    }
}
