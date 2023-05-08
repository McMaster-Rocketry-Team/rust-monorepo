use core::fmt;

use crate::utils::io_traits::AsyncReader;

use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn open_file_for_read(&self, file_id: u64) -> Option<FileReader<F, C>> {
        let mut at = self.allocation_table.write().await;
        if let Some(file_entry) = self.find_file_entry_mut(&mut at.allocation_table, file_id) {
            if file_entry.opened {
                return None;
            }
            file_entry.opened = true;

            return Some(FileReader::new(self, &file_entry));
        }
        None
    }
}

pub struct FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    current_sector_index: Option<u16>,
    current_page_index: u16,
    sector_data_length: Option<u16>,

    sector_read_data_length: u16,
    file_id: u64,

    page_buffer: [u8; 5 + PAGE_SIZE],
    page_buffer_read_ahead_range: (usize, usize),

    closed: bool,
}

impl<'a, F, C> FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>, file_entry: &FileEntry) -> Self {
        Self {
            vlfs,
            sector_data_length: None,
            sector_read_data_length: 0,
            current_sector_index: file_entry.first_sector_index,
            current_page_index: 0,
            file_id: file_entry.file_id,
            page_buffer: [0u8; 5 + PAGE_SIZE],
            page_buffer_read_ahead_range: (0, 0),
            closed: false,
        }
    }

    fn set_current_sector_index(&mut self, sector_index: u16) {
        self.current_sector_index = if sector_index == 0xFFFF {
            None
        } else {
            Some(sector_index)
        };
        self.current_page_index = 0;
        self.sector_data_length = None;
        self.sector_read_data_length = 0;
    }

    // return is end of file
    async fn read_next_page(&mut self) -> Result<bool, VLFSError<F>> {
        if let Some(current_sector_index) = self.current_sector_index {
            let is_last_page = self.current_page_index == 15;
            let mut flash = self.vlfs.flash.lock().await;
            let sector_address = current_sector_index as usize * SECTOR_SIZE;

            if self.sector_data_length.is_none() {
                defmt::assert!(self.current_page_index == 0);
                let sector_data_length_address = (sector_address + SECTOR_SIZE - 8 - 8) as u32;
                let read_result = flash
                    .read(sector_data_length_address, 8, &mut self.page_buffer)
                    .await
                    .map_err(VLFSError::from_flash)?;
                self.sector_data_length = Some(find_most_common_u16_out_of_4(read_result).unwrap());
            }

            let sector_unread_data_length =
                self.sector_data_length.unwrap() - self.sector_read_data_length;

            if sector_unread_data_length == 0 {
                let next_sector_index_address = (sector_address + SECTOR_SIZE - 8) as u32;
                let read_result = flash
                    .read(next_sector_index_address, 8, &mut self.page_buffer)
                    .await
                    .map_err(VLFSError::from_flash)?;

                let next_sector_index = find_most_common_u16_out_of_4(read_result).unwrap();
                self.set_current_sector_index(next_sector_index);
                self.page_buffer_read_ahead_range = (0, 0);
                return Ok(false);
            }

            let read_data_length = core::cmp::min(
                sector_unread_data_length,
                if is_last_page {
                    PAGE_SIZE - 8 - 8 - 4
                } else {
                    PAGE_SIZE - 4
                } as u16,
            ) as usize;
            let read_data_length_padded = (read_data_length + 3) & !3;

            let page_address =
                (sector_address + self.current_page_index as usize * PAGE_SIZE) as u32;

            info!(
                "Read page {:X}, page_index: {}",
                page_address, self.current_page_index
            );

            let read_result = flash
                .read(
                    page_address,
                    if is_last_page {
                        256
                    } else {
                        read_data_length_padded + 4
                    },
                    &mut self.page_buffer,
                )
                .await
                .map_err(VLFSError::from_flash)?;
            self.sector_read_data_length += read_data_length as u16;
            drop(flash);

            // info!("read result: len: {} {=[u8]:X}", read_result.len(), read_result);
            let data_buffer_padded = &read_result[..read_data_length_padded];
            let expected_crc_buffer =
                &read_result[read_data_length_padded..(read_data_length_padded + 4)];
            let expected_crc = u32::from_be_bytes(expected_crc_buffer.try_into().unwrap());
            let mut crc = self.vlfs.crc.lock().await;
            let actual_crc = crc.calculate(data_buffer_padded);
            drop(crc);

            if actual_crc != expected_crc {
                info!(
                    "CRC mismatch: expected {}, actual {}",
                    expected_crc, actual_crc
                );
                return Err(VLFSError::CorruptedPage {
                    address: page_address,
                });
            }

            if is_last_page {
                let next_sector_index =
                    find_most_common_u16_out_of_4(&read_result[(PAGE_SIZE - 8)..]).unwrap();
                self.set_current_sector_index(next_sector_index);
            } else {
                self.current_page_index += 1;
            }

            self.page_buffer_read_ahead_range = (5, 5 + read_data_length);
            return Ok(false);
        } else {
            self.page_buffer_read_ahead_range = (0, 0);
            return Ok(true);
        }
    }

    pub async fn close(mut self) {
        let mut at = self.vlfs.allocation_table.write().await;
        let file_entry = self
            .vlfs
            .find_file_entry_mut(&mut at.allocation_table, self.file_id)
            .unwrap();
        file_entry.opened = false;
        self.closed = true;
    }
}

impl<'a, F, C> AsyncReader for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Error = VLFSError<F>;

    async fn read_slice<'b>(
        &mut self,
        read_buffer: &'b mut [u8],
        length: usize,
    ) -> Result<&'b [u8], VLFSError<F>> {
        let mut read_length = 0;

        while read_length < length {
            let read_ahead_slice = &self.page_buffer
                [self.page_buffer_read_ahead_range.0..self.page_buffer_read_ahead_range.1];
            if read_ahead_slice.len() == 0 {
                match self.read_next_page().await {
                    Ok(false) => {
                        continue;
                    }
                    Ok(true) => {
                        return Ok(&read_buffer[..read_length]);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let unread_length = length - read_length;
            if read_ahead_slice.len() <= unread_length {
                (&mut read_buffer[read_length..(read_length + read_ahead_slice.len())])
                    .copy_from_slice(read_ahead_slice);

                read_length += read_ahead_slice.len();
                self.page_buffer_read_ahead_range = (0, 0);
            } else {
                (&mut read_buffer[read_length..length])
                    .copy_from_slice(&read_ahead_slice[..unread_length]);

                read_length += unread_length;
                self.page_buffer_read_ahead_range.0 += unread_length;
            }
        }

        Ok(&read_buffer[..length])
    }
}

impl<'a, F, C> Drop for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn drop(&mut self) {
        if !self.closed {
            defmt::panic!(
                "FileReader for file {:X} dropped without being closed",
                self.file_id
            );
        }
    }
}

impl<'a, F, C> fmt::Debug for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReader")
            .field("file_id", &self.file_id)
            .finish()
    }
}
