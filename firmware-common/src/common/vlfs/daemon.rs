use super::*;
use crate::{
    driver::{crc::Crc, flash::SpiFlash},
};

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    async fn flush_single(&self, entry:WritingQueueEntry){
        let mut flash = self.flash.lock().await;
        match entry {
            WritingQueueEntry::EraseSector { address }=>{
                flash.erase_sector_4kib(address).await;
            }
            WritingQueueEntry::WritePage { address,mut data } =>{
                flash.write_256b(address, &mut data).await;
            },
        }
    }

    pub async fn flush(&mut self) {
        loop {
            let data = self.writing_queue.receiver().try_recv();
            let entry = if data.is_err() {
                return;
            } else {
                data.unwrap()
            };

            self.flush_single(entry).await;
        }
    }
}
