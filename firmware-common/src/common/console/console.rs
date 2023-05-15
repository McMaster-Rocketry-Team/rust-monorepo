use crate::common::device_manager::prelude::*;
use crate::{
    device_manager_type,
    driver::{
        buzzer::Buzzer, device_management::DeviceManagement, imu::IMU, serial::Serial, timer::Timer,
    },
    try_or_warn,
};
use defmt::info;

use vlfs::{Crc, Flash, VLFS};

use super::programs::{
    benchmark_flash::BenchmarkFlash, calibrate::Calibrate, change_mode::ChangeMode,
    read_file::ReadFile, read_nyoom::ReadNyoom, write_file::WriteFile,
};

pub async fn run_console(
    fs: &VLFS<impl Flash, impl Crc>,
    mut serial: impl Serial,
    device_manager: device_manager_type!(),
) -> ! {
    let write_file = WriteFile::new();
    let read_nyoom = ReadNyoom::new();
    let benchmark_flash = BenchmarkFlash::new();
    let change_mode = ChangeMode::new();
    let read_file = ReadFile::new();
    let calibrate = Calibrate::new();

    let mut command_buffer = [0u8; 8];
    loop {
        if serial.read_all(&mut command_buffer).await.is_err() {
            continue;
        };
        let command_id = u64::from_be_bytes(command_buffer);

        if command_id == write_file.id() {
            try_or_warn!(write_file.start(&mut serial, fs).await);
        } else if command_id == read_nyoom.id() {
            try_or_warn!(read_nyoom.start(&mut serial, fs).await);
        } else if command_id == benchmark_flash.id() {
            try_or_warn!(
                benchmark_flash
                    .start(&mut serial, fs, device_manager.timer)
                    .await
            );
        } else if command_id == change_mode.id() {
            change_mode.start(&mut serial, fs, device_manager).await
        } else if command_id == read_file.id() {
            try_or_warn!(read_file.start(&mut serial, fs).await);
        } else if command_id == calibrate.id() {
            try_or_warn!(calibrate.start(&mut serial, fs, device_manager,).await);
        } else {
            info!("Unknown command: {:X}", command_id);
        }
    }
}
