use crate::{
    driver::{serial::Serial, timer::Timer},
    try_or_warn,
};
use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use vlfs::{Crc, Flash, VLFS};

use super::programs::{
    benchmark_flash::BenchmarkFlash, read_nyoom::ReadNyoom, write_file::WriteFile,
};

pub struct Console<'a, I: Timer, T: Serial, F: Flash, C: Crc>
where
    F::Error: defmt::Format,
    F: defmt::Format,
{
    timer: I,
    serial: Mutex<CriticalSectionRawMutex, T>,
    vlfs: &'a VLFS<F, C>,
}

impl<'a, I: Timer, T: Serial, F: Flash, C: Crc> Console<'a, I, T, F, C>
where
    F::Error: defmt::Format,
    F: defmt::Format,
{
    pub fn new(timer: I, serial: T, vlfs: &'a VLFS<F, C>) -> Self {
        Self {
            timer,
            serial: Mutex::new(serial),
            vlfs,
        }
    }

    pub async fn run(&mut self) -> ! {
        let write_file = WriteFile::new();
        let read_nyoom = ReadNyoom::new();
        let benchmark_flash = BenchmarkFlash::new();
        let mut serial = self.serial.lock().await;
        let mut command_buffer = [0u8; 8];

        loop {
            if serial.read_all(&mut command_buffer).await.is_err() {
                continue;
            };
            let command_id = u64::from_be_bytes(command_buffer);

            if command_id == write_file.id() {
                try_or_warn!(write_file.start(&mut serial, &self.vlfs).await);
            } else if command_id == read_nyoom.id() {
                try_or_warn!(read_nyoom.start(&mut serial, &self.vlfs).await);
            } else if command_id == benchmark_flash.id() {
                try_or_warn!(
                    benchmark_flash
                        .start(&mut serial, &self.vlfs, &self.timer)
                        .await
                );
            } else {
                info!("Unknown command: {:X}", command_id);
            }
        }
    }
}
