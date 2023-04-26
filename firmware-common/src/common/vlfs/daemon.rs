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
            WritingQueueEntry::WritePage { address, mut data } => {
                info!("Flush: write {:#X}", address);
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
