use defmt::{info, unwrap};
use vlfs::{io_traits::AsyncReader, Crc, Flash, VLFS};

use crate::driver::serial::Serial;

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
    ) -> Result<(), ()>
    where
        F::Error: defmt::Format,
        F: defmt::Format,
    {
        let mut buffer = [0u8; 64];
        unwrap!(serial.read_all(&mut buffer[..8]).await);
        let file_id = u64::from_be_bytes((&buffer[0..8]).try_into().unwrap());

        let mut reader = unwrap!(vlfs.open_file_for_read(file_id).await);

        loop {
            unwrap!(serial.read_all(&mut buffer[0..1]).await);
            info!("Read next");
            let nyoom_message_length_bytes = unwrap!(reader.read_slice(&mut buffer[0..4], 4).await);
            if nyoom_message_length_bytes.len() == 0 {
                unwrap!(serial.write(&[0u8; 4]).await);
                reader.close().await;
                return Ok(());
            }
            let nyoom_message_length =
                u32::from_be_bytes(nyoom_message_length_bytes.try_into().unwrap());

            unwrap!(serial.write(nyoom_message_length_bytes).await);
            let mut length_read = 0u32;
            while length_read < nyoom_message_length {
                let read_chunk_size =
                    core::cmp::min(buffer.len() as u32, nyoom_message_length - length_read);
                let buffer = unwrap!(
                    reader
                        .read_slice(&mut buffer, read_chunk_size as usize)
                        .await
                );
                unwrap!(serial.write(buffer).await);
                length_read += read_chunk_size;
            }
        }
    }
}
