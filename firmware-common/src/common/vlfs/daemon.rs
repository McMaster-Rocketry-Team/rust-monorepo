use super::*;
use crate::driver::{crc::Crc, flash::SpiFlash};

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    async fn flush_single(&self, entry: WritingQueueEntry) {
        let mut flash = self.flash.lock().await;
        match entry {
            WritingQueueEntry::EraseSector { address } => {
                info!("Flush: erase {:#X}", address);
                flash.erase_sector_4kib(address).await;
            }
            WritingQueueEntry::WritePage {
                address,
                crc_offset,
                mut data,
            } => {
                info!("Flush: write {:#X}", address);
                if let Some(crc_offset) = crc_offset {
                    let mut crc = self.crc.lock().await;
                    let crc = crc.calculate(&data[..crc_offset]);
                    (&mut data[crc_offset..(crc_offset + 4)]).copy_from_slice(&crc.to_be_bytes());
                }
                flash.write_256b(address, &mut data).await;
            }
        }
    }

    pub async fn flush(&self) {
        loop {
            let entry = self.writing_queue.receiver().recv().await;
            self.flush_single(entry).await;
        }
    }
}
