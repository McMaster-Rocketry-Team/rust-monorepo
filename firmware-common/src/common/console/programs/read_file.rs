use defmt::{unwrap, warn};
use vlfs::{io_traits::AsyncReader, Crc, Flash, VLFSReadStatus, VLFS};

use crate::driver::serial::Serial;

// TODO implement `ConsoleProgram` and add to `start_common_programs`
pub struct ReadFile {}

impl ReadFile {
    pub fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x4
    }

    pub async fn start<T: Serial, F: Flash, C: Crc>(
        &self,
        serial: &mut T,
        vlfs: &VLFS<F, C>,
    ) -> Result<(), ()> {
        let mut buffer = [0u8; 64];
        unwrap!(serial.read_all(&mut buffer[..8]).await);
        let file_id = u64::from_be_bytes((&buffer[0..8]).try_into().unwrap()).into();

        let mut reader = unwrap!(vlfs.open_file_for_read(file_id).await);
        loop {
            let read_result = reader.read_all(&mut buffer).await;
            if let Ok((buffer, VLFSReadStatus::EndOfFile)) = read_result {
                unwrap!(serial.write(buffer).await);
                reader.close().await;
                return Ok(());
            } else if let Ok((buffer, VLFSReadStatus::CorruptedPage { address })) = read_result {
                warn!("Corrupted page at address {}", address);
                unwrap!(serial.write(buffer).await);
            } else if let Ok((buffer, _)) = read_result {
                unwrap!(serial.write(buffer).await);
            }
        }
    }
}
