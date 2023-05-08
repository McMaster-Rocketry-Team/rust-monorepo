use defmt::{info, unwrap};
use vlfs::{
    io_traits::{AsyncWriter},
    Crc, Flash, VLFS,
};

use crate::driver::serial::Serial;
pub struct WriteFile {}

impl WriteFile {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x0
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
        unwrap!(serial.read_all(&mut buffer[..(8 + 2 + 4)]).await);
        let file_id = u64::from_be_bytes((&buffer[0..8]).try_into().unwrap());
        let file_type = u16::from_be_bytes((&buffer[8..10]).try_into().unwrap());
        let file_size = u32::from_be_bytes((&buffer[10..14]).try_into().unwrap());

        if vlfs.create_file(file_id, file_type).await.is_err() {
            unwrap!(vlfs.remove_file(file_id).await);
            unwrap!(vlfs.create_file(file_id, file_type).await);
        }

        info!(
            "File created with id: {:X}, total size: {}",
            file_id, file_size
        );

        let mut file = unwrap!(vlfs.open_file_for_write(file_id).await);

        let mut wrote_len = 0u32;
        while wrote_len < file_size {
            // info!("Wrote len: {}", wrote_len);
            let chunk_len = core::cmp::min(buffer.len() as u32, file_size - wrote_len);
            // info!("sending chunk len: {}", chunk_len);
            unwrap!(serial.write(&chunk_len.to_be_bytes()).await);

            // info!("reading chunk");
            let read_len = unwrap!(serial.read(&mut buffer).await);
            assert!(read_len == chunk_len as usize);

            unwrap!(
                file.extend_from_slice(&buffer[..(chunk_len as usize)])
                    .await
            );
            wrote_len += chunk_len;
        }

        unwrap!(file.close().await);

        let (size, sector) = unwrap!(vlfs.get_file_size(file_id).await);
        info!("File saved! size: {}, sector: {}", size, sector);

        Ok(())
    }
}
