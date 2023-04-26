use crate::{driver::{flash::SpiFlash, crc::Crc}, common::io_traits::Writer};
use super::*;

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    async fn flush_single(&self, mut entry: WritingQueueEntry) {
        let new_sector_index = self.find_avaliable_sector().await.unwrap(); // FIXME handle error

        let mut opened_files = self.opened_files.write().await;
        if let Some(opened_file) = &mut opened_files[entry.fd] {
            // save the new sector index to the end of the last sector
            let was_empty = if let Some(last_sector_index) = opened_file.current_sector_index {
                let last_sector_next_sector_address =
                    (last_sector_index as usize * SECTOR_SIZE + (4096 - 256)) as u32;
                new_write_buffer!(write_buffer, 256);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                self.flash
                    .lock()
                    .await
                    .write_256b(last_sector_next_sector_address, write_buffer.as_mut_slice())
                    .await;
                false
            } else {
                let mut at = self.allocation_table.write().await;
                for file_entry in &mut at.allocation_table.file_entries {
                    if file_entry.file_id == opened_file.file_entry.file_id {
                        opened_file.file_entry.first_sector_index = Some(new_sector_index);
                        file_entry.first_sector_index = Some(new_sector_index);
                        break;
                    }
                }
                true
            };

            // write the data to new sector
            let write_sector_address = if entry.overwrite_sector && let Some(current_sector_index) = opened_file.current_sector_index {
                (current_sector_index as usize * SECTOR_SIZE) as u32
            } else {
                let mut free_sectors = self.free_sectors.write().await;
                free_sectors
                    .as_mut_bitslice()
                    .set(new_sector_index as usize, true);
                opened_file.current_sector_index = Some(new_sector_index);
                (new_sector_index as usize * SECTOR_SIZE) as u32
            };

            if was_empty {
                self.write_allocation_table().await;
            }

            // erase old data
            let mut flash = self.flash.lock().await;
            flash.erase_sector_4kib(write_sector_address).await;

            // put crc to the buffer
            {
                let mut write_buffer = WriteBuffer::new(&mut entry.data, 5 + 8);
                write_buffer.set_offset(entry.data_length as usize);
                write_buffer.align_4_bytes();
                let mut crc = self.crc.lock().await;
                let crc = crc.calculate(write_buffer.as_slice_without_start());
                write_buffer.extend_from_u32(crc);
            }

            // put length of the data to the buffer
            {
                let mut write_buffer = WriteBuffer::new(&mut entry.data, 5);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
            }

            // write buffer to flash
            flash
                .write(
                    write_sector_address,
                    (8 + entry.data_length + 4) as usize,
                    &mut entry.data,
                )
                .await;
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