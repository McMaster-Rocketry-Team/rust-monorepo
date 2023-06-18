use defmt::{info, unwrap};
use vlfs::{io_traits::AsyncReader, Crc, Flash, VLFSReadStatus, VLFS};

use crate::driver::serial::Serial;

// TODO implement `ConsoleProgram` and add to `start_common_programs`
pub struct ReadNyoom {}

impl ReadNyoom {
    pub fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x1
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
            unwrap!(serial.read_all(&mut buffer[0..1]).await);
            info!("Read next");
            let read_result = reader.read_u32(&mut buffer).await;
            if let Ok((Some(nyoom_message_length), _)) = read_result {
                unwrap!(serial.write(&nyoom_message_length.to_be_bytes()).await);

                let mut length_read = 0u32;
                while length_read < nyoom_message_length {
                    let read_chunk_size =
                        core::cmp::min(buffer.len() as u32, nyoom_message_length - length_read);
                    let buffer = unwrap!(
                        reader
                            .read_slice(&mut buffer, read_chunk_size as usize)
                            .await
                    )
                    .0;
                    unwrap!(serial.write(buffer).await);
                    length_read += read_chunk_size;
                }
            } else if let Ok((None, VLFSReadStatus::EndOfFile)) = read_result {
                unwrap!(serial.write(&[0u8; 4]).await);
                reader.close().await;
                return Ok(());
            } else {
                panic!("Failed to read nyoom message length")
            }
        }
    }
}
