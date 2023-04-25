use crate::{
    common::{
        buffer::WriteBuffer,
        vlfs::{WritingQueueEntry, VLFS},
    },
    driver::{crc::Crc, flash::SpiFlash, pyro::PyroChannel, serial::Serial, timer::Timer},
    heapless_format_bytes,
};
use defmt::info;
use heapless::String;
use heapless::Vec;

pub struct Console<I: Timer, T: Serial, F: SpiFlash, C: Crc, P: PyroChannel> {
    timer: I,
    serial: T,
    vlfs: VLFS<F, C>,
    pyro: P,
}

impl<I: Timer, T: Serial, F: SpiFlash, C: Crc, P: PyroChannel> Console<I, T, F, C, P> {
    pub fn new(timer: I, serial: T, vlfs: VLFS<F, C>, pyro: P) -> Self {
        Self {
            timer,
            serial,
            vlfs,
            pyro,
        }
    }

    async fn read_command(&mut self) -> Result<String<64>, ()> {
        self.serial.write(b"vlf3> ").await?;
        let mut buffer = [0u8; 64];
        let mut command = String::<64>::new();

        loop {
            let read_len = self.serial.read(&mut buffer).await?;
            if read_len == 0 {
                continue;
            }

            for byte in (&buffer[0..read_len]).iter() {
                if *byte == b'\r' {
                    self.serial.write(b"\r\n").await?;
                    return Ok(command);
                }

                if *byte == 8 {
                    if command.pop().is_some() {
                        self.serial.write(&[8, b' ', 8]).await?;
                    }
                } else {
                    command.push(*byte as char).unwrap();
                    self.serial.write(&[*byte]).await?;
                }

                info!("command: {}", command.as_str());
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        loop {
            let command = self.read_command().await?;
            let command = command.as_str();
            let command = command.split_ascii_whitespace().collect::<Vec<&str, 10>>();
            if command.len() == 0 {
                continue;
            }
            if command[0] == "fs.ls" {
                // let (file_count, files_iter) = self.vlfs.list_files().await;
                // self.serial
                //     .writeln(heapless_format_bytes!(64, "{} files:", file_count))
                //     .await?;
                // for file in files_iter {
                //     let (size, sectors) = self.vlfs.get_file_size(file.file_id).await.unwrap();
                //     self.serial
                //         .writeln(heapless_format_bytes!(
                //             64,
                //             "ID: {:#18X}  type: {:#6X}  size: {}  sectors: {}",
                //             file.file_id,
                //             file.file_type,
                //             size,
                //             sectors,
                //         ))
                //         .await?;
                // }
            } else if command[0] == "fs.touch" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                let typ = u16::from_str_radix(command[2], 16).unwrap();
                self.vlfs.create_file(id, typ).await.unwrap();
                self.serial.writeln(b"File created").await?;
            } else if command[0] == "fs.rm" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                let result = self.vlfs.remove_file(id).await;
                if result.is_err() {
                    self.serial.writeln(b"File is in use").await?;
                } else {
                    self.serial.writeln(b"File removed").await?;
                }
            } else if command[0] == "fs.flush" {
                self.vlfs.flush().await;
                self.serial.writeln(b"Flushed").await?;
            } else if command[0] == "fs.open" {
                let id = u64::from_str_radix(command[1], 16).unwrap();
                let fd = self.vlfs.open_file(id).await.unwrap();
                self.serial
                    .writeln(heapless_format_bytes!(32, "File Descriptor: {:#X}", fd))
                    .await?;
            } else if command[0] == "fs.write" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let mut entry = WritingQueueEntry::new(fd);
                let mut write_buffer = WriteBuffer::new(&mut entry.data, 5 + 8);
                write_buffer.extend_from_slice(command[2].as_bytes());
                entry.data_length = write_buffer.len() as u16;
                self.vlfs.write_file(entry).await;
                self.serial.writeln(b"Added to queue").await?;
            } else if command[0] == "fs.read" {
                let fd = usize::from_str_radix(command[1], 16).unwrap();
                let mut buffer = [0u8; 64];
                let result = self.vlfs.read_file(fd, &mut buffer).await.unwrap();
                self.serial.writeln(result).await?;
            } else if command[0] == "sys.reset" {
                // cortex_m::peripheral::SCB::sys_reset();
            } else if command[0] == "f" {
                self.pyro.set_enable(true).await;
                self.timer.sleep(100).await;
                self.pyro.set_enable(false).await;
                self.serial.writeln(b"Pyro fired!").await?;
            } else {
                self.serial.writeln(b"Unknown command!").await?;
            }
        }
    }
}
