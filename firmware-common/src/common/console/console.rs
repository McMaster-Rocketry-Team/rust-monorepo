use defmt::info;
use heapless::String;

use crate::driver::serial::Serial;

use super::programs::{hello::Hello, program::ConsoleProgram};

pub struct Console<T: Serial> {
    serial: T,
}

impl<T: Serial> Console<T> {
    pub fn new(serial: T) -> Self {
        Self { serial }
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
                self.serial.write(&[*byte]).await?;
                command.push(*byte as char).unwrap();
                info!("command: {}", command.as_str());
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        self.serial
            .write(b"Welcome to Void Lake Fusion 3 Console!\r\n")
            .await?;
        let hello_program = Hello::<T>::new();

        loop {
            let command = self.read_command().await?;
            let command = command.as_str();
            if command == hello_program.name() {
                hello_program.start(&mut self.serial).await?;
            } else {
                self.serial.write(b"Unknown command!\r\n").await?;
            }
        }
    }
}
