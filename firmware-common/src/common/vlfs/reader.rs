use core::fmt;

use super::*;
use crate::common::io_traits::AsyncReader;

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub async fn open_file_for_read(&self, file_id: u64) -> Option<FileReader<F, C>> {
        let mut at = self.allocation_table.write().await;
        if let Some(file_entry) = self.find_file_entry_mut(&mut at.allocation_table, file_id) {
            if file_entry.opened {
                return None;
            }
            file_entry.opened = true;

            let first_sector_data_length: u16 =
                if let Some(sector_index) = file_entry.first_sector_index {
                    // TODO read data length and next sector index in one go
                    let mut flash = self.flash.lock().await;
                    let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
                    let sector_data_length_address = sector_address + 4096 - 256 - 8;

                    let mut read_buffer = [0u8; 5 + 8];
                    flash
                        .read(sector_data_length_address, 8, &mut read_buffer)
                        .await;
                    find_most_common_u16_out_of_4(&read_buffer[5..]).unwrap()
                } else {
                    0
                };

            return Some(FileReader::new(self, first_sector_data_length, &file_entry));
        }
        None
    }
}

pub struct FileReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    sector_data_length: u16,
    sector_read_data_length: u16,
    current_sector_index: Option<u16>,
    file_id: u64,
}

impl<'a, F, C> fmt::Debug for FileReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReader")
            .field("file_id", &self.file_id)
            .finish()
    }
}


impl<'a, F, C> FileReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>, first_sector_data_length: u16, file_entry: &FileEntry) -> Self {
        Self {
            vlfs,
            sector_data_length: first_sector_data_length,
            sector_read_data_length: 0,
            current_sector_index: file_entry.first_sector_index,
            file_id: file_entry.file_id,
        }
    }
}

impl<'a, F, C> AsyncReader for FileReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    async fn read_slice<'b>(&mut self, buffer: &'b mut [u8], length: usize) -> &'b [u8] {
        buffer
    }
}
