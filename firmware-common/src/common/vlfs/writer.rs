use crate::common::io_traits::Writer;

use super::utils::CopyFromU16x4;
use super::*;

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub async fn open_file_for_write(&self, file_id: u64) -> Option<FileWriter<F, C>> {
        let mut at = self.allocation_table.write().await;
        if let Some(file_entry) = self.find_file_entry_mut(&mut at.allocation_table, file_id) {
            if file_entry.opened {
                return None;
            }
            file_entry.opened = true;

            let new_file = if file_entry.first_sector_index.is_none() {
                let new_sector_index = self.claim_avaliable_sector().unwrap(); // FIXME handle error
                file_entry.first_sector_index = Some(new_sector_index);
                true
            } else {
                false
            };

            let file_entry_clone = file_entry.clone();
            drop(at);

            if new_file {
                self.write_allocation_table().await;
            }

            return Some(FileWriter::new(self, new_file, file_entry_clone));
        }

        None
    }
}

pub struct FileWriter<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    buffer: [u8; 5 + 256],
    buffer_offset: usize,
    sector_data_length: u16,
    current_sector_index: u16,
    file_id: u64,
    new_file: bool,
}

impl<'a, F, C> FileWriter<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>, new_file: bool, file_entry: FileEntry) -> Self {
        FileWriter {
            vlfs,
            buffer: [0xFFu8; 5 + 256],
            buffer_offset: 5,
            sector_data_length: 0,
            current_sector_index: file_entry.first_sector_index.unwrap(),
            file_id: file_entry.file_id,
            new_file,
        }
    }

    fn send_page_to_queue(&mut self, address: u32) {
        self.vlfs
            .writing_queue
            .try_send(WritingQueueEntry::WritePage {
                address,
                data: self.buffer,
            })
            .unwrap();
        self.buffer = [0xFFu8; 5 + 256];
        self.buffer_offset = 5;
    }

    fn send_erase_to_queue(&mut self, address: u32) {
        self.vlfs
            .writing_queue
            .try_send(WritingQueueEntry::EraseSector { address })
            .unwrap();
    }

    fn is_last_data_page(&self) -> bool {
        self.sector_data_length > 4096 - 512
    }

    fn page_address(&self) -> u32 {
        let current_sector_address = (self.current_sector_index as usize * SECTOR_SIZE) as u32;
        current_sector_address + ((self.sector_data_length as u32 - 1) & !255)
    }

    fn write_crc_and_length(&mut self, crc: u32) {
        self.buffer_offset = 256 - 8 - 4;
        (&mut self.buffer[self.buffer_offset..(self.buffer_offset + 4)])
            .copy_from_slice(&crc.to_be_bytes());
        (&mut self.buffer[self.buffer_offset + 4..]).copy_from_u16x4(self.sector_data_length);
    }

    pub async fn flush(&mut self) {
        if self.sector_data_length == 0 {
            return;
        }

        // pad to 4 bytes
        let old_sector_data_length = self.sector_data_length;
        self.sector_data_length = (self.sector_data_length + 3) & !3;
        self.buffer_offset += (self.sector_data_length - old_sector_data_length) as usize;

        if self.is_last_data_page() {
            // last data page contains data
            self.write_crc_and_length(0); // TODO

            self.send_page_to_queue(self.page_address());
        } else {
            // last data page does not contain data
            self.send_page_to_queue(self.page_address());

            self.write_crc_and_length(0); // TODO

            let current_sector_address = (self.current_sector_index as usize * SECTOR_SIZE) as u32;
            self.send_page_to_queue(current_sector_address + (4096 - 512));
        }

        self.sector_data_length = 0;
    }

    pub async fn close(mut self) {
        self.flush().await;

        let mut at = self.vlfs.allocation_table.write().await;
        let file_entry = self
            .vlfs
            .find_file_entry_mut(&mut at.allocation_table, self.file_id)
            .unwrap();
        file_entry.opened = false;
        if self.new_file {
            // no data has been written since the creation of this file
            self.vlfs
                .return_sector(file_entry.first_sector_index.take().unwrap());
        }
    }
}

impl<'a, F, C> Writer for FileWriter<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    fn extend_from_slice(&mut self, slice: &[u8]) {
        let mut slice = slice;
        while slice.len() > 0 {
            // save the new sector index
            if self.sector_data_length == 0 {
                if self.new_file {
                    // if this is a new file, this step has already been done in `open_file_for_write`
                    self.new_file = false;
                } else {
                    // save the new sector index to the end of last sector
                    let last_sector_next_sector_address =
                        (self.current_sector_index as usize * SECTOR_SIZE + (4096 - 256)) as u32;
                    let new_sector_index = self.vlfs.claim_avaliable_sector().unwrap();
                    self.current_sector_index = new_sector_index;
                    (&mut self.buffer[self.buffer_offset..]).copy_from_u16x4(new_sector_index);
                    self.send_page_to_queue(last_sector_next_sector_address);
                }
                self.send_erase_to_queue((self.current_sector_index as usize * SECTOR_SIZE) as u32);
            }

            let buffer_free = if self.is_last_data_page() {
                self.buffer.len() - self.buffer_offset - 8 - 4
            } else {
                self.buffer.len() - self.buffer_offset
            };

            if slice.len() < buffer_free {
                (&mut self.buffer[self.buffer_offset..(self.buffer_offset + slice.len())])
                    .copy_from_slice(slice);
                self.buffer_offset += slice.len();
                self.sector_data_length += slice.len() as u16;
            } else {
                (&mut self.buffer[self.buffer_offset..]).copy_from_slice(&slice[..buffer_free]);
                self.buffer_offset += buffer_free;
                self.sector_data_length += buffer_free as u16;

                if self.is_last_data_page() {
                    self.write_crc_and_length(0); // TODO

                    self.send_page_to_queue(self.page_address());
                    self.sector_data_length = 0;
                } else {
                    self.send_page_to_queue(self.page_address());
                }

                slice = &slice[buffer_free..];
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum WritingQueueEntry {
    WritePage { address: u32, data: [u8; 5 + 256] },
    EraseSector { address: u32 },
}
