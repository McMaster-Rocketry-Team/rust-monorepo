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
                let new_sector_index = self.find_avaliable_sector().unwrap(); // FIXME handle error
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
    file_entry: FileEntry,
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
            new_file,
            file_entry,
        }
    }

    fn send_to_queue(&mut self, address: u32) {
        self.vlfs
            .page_writing_queue
            .try_send(PageWritingQueueEntry {
                address,
                data: self.buffer,
            })
            .unwrap();
        self.buffer = [0xFFu8; 5 + 256];
        self.buffer_offset = 5;
    }

    fn is_last_page(&self) -> bool {
        self.sector_data_length > 4096 - 512
    }

    async fn flush(&mut self) {}
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
                    let new_sector_index = self.vlfs.find_avaliable_sector().unwrap();
                    self.current_sector_index = new_sector_index;
                    (&mut self.buffer[self.buffer_offset..]).copy_from_u16x4(new_sector_index);
                    self.send_to_queue(last_sector_next_sector_address);
                }
            }

            let buffer_free = if self.is_last_page() {
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
                self.buffer_offset += slice.len();
                self.sector_data_length += slice.len() as u16;

                if self.is_last_page() {
                    let crc = 0u32;
                    (&mut self.buffer[self.buffer_offset..(self.buffer_offset + 4)])
                        .copy_from_slice(&crc.to_be_bytes());
                    (&mut self.buffer[self.buffer_offset + 4..])
                        .copy_from_u16x4(self.sector_data_length);
                    self.sector_data_length = 0;
                }
                self.send_to_queue(0);

                slice = &slice[buffer_free..];
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct PageWritingQueueEntry {
    address: u32,
    data: [u8; 5 + 256],
}
