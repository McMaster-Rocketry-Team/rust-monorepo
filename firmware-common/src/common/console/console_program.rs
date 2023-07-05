use futures::future::join;

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

pub async fn start_console_program_2<P: ConsoleProgram + Clone, const N: usize>(
    device_manager: device_manager_type!(),
    console1: &Console<impl Serial, N>,
    console2: &Console<impl Serial, N>,
    program: P,
) -> ! {
    let console1_program_fut = start_console_program(device_manager, console1, program.clone());
    let console2_program_fut = start_console_program(device_manager, console2, program);
    join(console1_program_fut, console2_program_fut).await;
    defmt::unreachable!()
}
