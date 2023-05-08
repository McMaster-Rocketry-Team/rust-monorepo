use super::*;
use crate::driver::{crc::Crc, flash::Flash};

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    // todo get rid of queue (the caller can't get errors), replace with fair rwlock
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
                info!("Flush: write to {:#X}", address);
                if let Some(crc_offset) = crc_offset {
                    let mut crc = self.crc.lock().await;
                    let crc = crc.calculate(&data[5..(crc_offset + 5)]);
                    (&mut data[(crc_offset + 5)..(crc_offset + 5 + 4)])
                        .copy_from_slice(&crc.to_be_bytes());
                }
                info!("Flush: {=[u8]:02X}", &data[5..]);
                flash.write_256b(address, &mut data).await;
            }
        }
    }

    pub async fn flush(&self) {
        let receiver = self.writing_queue.receiver();
        loop {
            let entry = receiver.recv().await;
            self.flush_single(entry).await;
        }
    }
}
