use crate::{
    common::{
        io_traits::Writer,
        vlfs::{reader::FileReader, writer::FileWriter, VLFS},
    },
    driver::{crc::Crc, flash::SpiFlash, pyro::PyroChannel, serial::Serial, timer::Timer},
    heapless_format_bytes,
};
use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use futures::future::join;
use heapless::String;
use heapless::Vec;

pub struct Console<I: Timer, T: Serial, F: SpiFlash, C: Crc, P: PyroChannel> {
    timer: I,
    serial: Mutex<CriticalSectionRawMutex, T>,
    vlfs: VLFS<F, C>,
    pyro: P,
}

impl<I: Timer, T: Serial, F: SpiFlash, C: Crc, P: PyroChannel> Console<I, T, F, C, P> {
    pub fn new(timer: I, serial: T, vlfs: VLFS<F, C>, pyro: P) -> Self {
        Self {
            timer,
            serial: Mutex::new(serial),
            vlfs,
            pyro,
        }
    }

    async fn write(&self, data: &[u8]) {
        let mut serial = self.serial.lock().await;
        serial.write(data).await.unwrap();
    }

    async fn writeln(&self, data: &[u8]) {
        self.write(data).await;
        self.write(b"\r\n").await;
    }

    async fn read(&self, buffer: &mut [u8]) -> usize {
        let mut serial = self.serial.lock().await;
        serial.read(buffer).await.unwrap()
    }

    async fn read_command(&self) -> Result<String<64>, ()> {
        self.write(b"vlf3> ").await;
        let mut buffer = [0u8; 64];
        let mut command = String::<64>::new();

        loop {
            let read_len = self.read(&mut buffer).await;
            if read_len == 0 {
                continue;
            }

            for byte in (&buffer[0..read_len]).iter() {
                if *byte == b'\r' {
                    self.write(b"\r\n").await;
                    return Ok(command);
                }

                if *byte == 8 {
                    if command.pop().is_some() {
                        self.write(&[8, b' ', 8]).await;
                    }
                } else {
                    command.push(*byte as char).unwrap();
                    self.write(&[*byte]).await;
                }
            }
        }
    }

    pub async fn run(&mut self) -> ! {
        let console_fut = self.run_console();
        let vlfs_fut = self.vlfs.flush();

        join(console_fut, vlfs_fut).await;

        loop {}
    }

    async fn run_console(&self) -> ! {
        let mut opened_write_files = Vec::<FileWriter<F, C>, 2>::new();
        let mut opened_read_files = Vec::<FileReader<F, C>, 2>::new();
        loop {
            let command = self.read_command().await.unwrap();
            let command = command.as_str();
            let command = command.split_ascii_whitespace().collect::<Vec<&str, 10>>();
            if command.len() == 0 {
                continue;
            }
            if command[0] == "fs.ls" {
                let files_iter = self.vlfs.files_iter().await;
                self.writeln(heapless_format_bytes!(64, "{} files:", files_iter.len()))
                    .await;
                for file in files_iter {
                    let (size, sectors) = self.vlfs.get_file_size(file.file_id).await.unwrap();
                    self.writeln(heapless_format_bytes!(
                        64,
                        "ID: {:#18X}  type: {:#6X}  size: {}  sectors: {}",
                        file.file_id,
                        file.file_type,
                        size,
                        sectors,
                    ))
                    .await;
                }
            } else if command[0] == "fs.test" {
                self.vlfs.create_file(0x1, 0x0).await.unwrap();
                let mut file_writer = self.vlfs.open_file_for_write(0x1).await.unwrap();
                file_writer.extend_from_slice(b"12345");
                file_writer.close().await;
            } else if command[0] == "fs.touch" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                let typ = u16::from_str_radix(command[2], 16).unwrap();
                self.vlfs.create_file(id, typ).await.unwrap();
                self.writeln(b"File created").await;
            } else if command[0] == "fs.rm" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                let result = self.vlfs.remove_file(id).await;
                if result.is_err() {
                    self.writeln(b"File is in use").await;
                } else {
                    self.writeln(b"File removed").await;
                }
            } else if command[0] == "fs.open.write" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                opened_write_files
                    .push(self.vlfs.open_file_for_write(id).await.unwrap())
                    .unwrap();
                self.writeln(heapless_format_bytes!(
                    32,
                    "File Descriptor: {:#X}",
                    opened_write_files.len() - 1
                ))
                .await;
            } else if command[0] == "fs.open.read" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                opened_read_files
                    .push(self.vlfs.open_file_for_read(id).await.unwrap())
                    .unwrap();
                self.writeln(heapless_format_bytes!(
                    32,
                    "File Descriptor: {:#X}",
                    opened_read_files.len() - 1
                ))
                .await;
            } else if command[0] == "fs.close.write" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let file_writer = opened_write_files.remove(fd);
                file_writer.close().await;
                self.writeln(b"Closed").await;
            } else if command[0] == "fs.close.read" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let file_reader = opened_read_files.remove(fd);
                file_reader.close().await;
                self.writeln(b"Closed").await;
            } else if command[0] == "fs.write" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let file_writer = opened_write_files.get_mut(fd).unwrap();
                file_writer.extend_from_slice(command[2].as_bytes());
                self.writeln(b"OK").await;
            } else if command[0] == "fs.write.flush" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let file_writer = opened_write_files.get_mut(fd).unwrap();
                file_writer.flush().await;
                self.writeln(b"OK").await;
            } else if command[0] == "fs.read" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let mut buffer = [0u8; 64];
                let file_reader = opened_read_files.get_mut(fd).unwrap();
                let result = file_reader.read_slice(&mut buffer, 64).await;
                self.writeln(result).await;
            } else if command[0] == "fs.rm" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                self.vlfs.remove_file(id).await.unwrap();
                self.writeln(b"Removed").await;
            } else if command[0] == "sys.reset" {
                // cortex_m::peripheral::SCB::sys_reset();
            } else if command[0] == "f" {
                // self.pyro.set_enable(true).await;
                // self.timer.sleep(100).await;
                // self.pyro.set_enable(false).await;
                // self.writeln(b"Pyro fired!").await;
            } else {
                self.writeln(b"Unknown command!").await;
            }
        }
    }
}
