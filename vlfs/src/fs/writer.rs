use core::fmt;

use crate::io_traits::AsyncWriter;

use super::utils::CopyFromU16x4;
use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn open_file_for_write(
        &self,
        file_id: u64,
    ) -> Result<FileWriter<F, C>, VLFSError<F::Error>> {
        let mut at = self.allocation_table.write().await;
        if let Some(file_entry) = self.find_file_entry_mut(&mut at.allocation_table, file_id) {
            if file_entry.opened {
                return Err(VLFSError::FileInUse);
            }
            file_entry.opened = true;

            let new_sector_index = self.claim_avaliable_sector_and_erase().await?;
            let new_file = if let Some(first_sector_index) = file_entry.first_sector_index {
                // this file has been written to before,
                // update "index of next sector" in the last sector

                // find index of the last sector
                let mut flash = self.flash.lock().await;
                let mut buffer = [0u8; 5 + PAGE_SIZE];
                let mut current_sector_index = first_sector_index;
                loop {
                    let next_sector_index_address =
                        (current_sector_index as usize * SECTOR_SIZE + SECTOR_SIZE - 8) as u32;

                    let read_result = flash
                        .read(next_sector_index_address, 8, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;
                    let next_sector_index = find_most_common_u16_out_of_4(read_result).unwrap();
                    if next_sector_index == 0xFFFF {
                        break;
                    } else {
                        current_sector_index = next_sector_index;
                    }
                }
                trace!("index of the last sector: {}", current_sector_index);

                let current_sector_address = (current_sector_index as usize * SECTOR_SIZE) as u32;

                let temp_sector_index = self.claim_avaliable_sector_and_erase().await?;
                // copy last sector to temp sector, with "index of next sector" changed
                for i in 0..PAGES_PER_SECTOR {
                    let read_address = (current_sector_address as usize + i * PAGE_SIZE) as u32;
                    flash
                        .read(read_address, PAGE_SIZE, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;
                    if i == PAGES_PER_SECTOR - 1 {
                        // last page
                        (&mut buffer[(5 + PAGE_SIZE - 8)..]).copy_from_u16x4(new_sector_index);
                    }

                    let write_address =
                        (temp_sector_index as usize * SECTOR_SIZE + i * PAGE_SIZE) as u32;
                    flash
                        .write_256b(write_address, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;
                }

                // erase last sector
                flash
                    .erase_sector_4kib(current_sector_address)
                    .await
                    .map_err(VLFSError::FlashError)?;

                // copy temp sector to last sector
                for i in 0..PAGES_PER_SECTOR {
                    let read_address =
                        (temp_sector_index as usize * SECTOR_SIZE + i * PAGE_SIZE) as u32;
                    flash
                        .read(read_address, PAGE_SIZE, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;

                    let write_address = (current_sector_address as usize + i * PAGE_SIZE) as u32;
                    flash
                        .write_256b(write_address, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;
                }
                self.return_sector(temp_sector_index).await;

                false
            } else {
                // this file haven't been written to before,
                // update allocation table
                file_entry.first_sector_index = Some(new_sector_index);
                true
            };

            let result = Ok(FileWriter::new(self, new_sector_index, file_entry.file_id));
            drop(at);

            if new_file {
                self.write_allocation_table().await?;
            }

            return result;
        }

        Err(VLFSError::FileDoesNotExist)
    }
}

pub struct FileWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    buffer: [u8; 5 + PAGE_SIZE],
    buffer_offset: usize,
    sector_data_length: u16,
    current_sector_index: u16,
    file_id: u64,

    closed: bool,
}

impl<'a, F, C> FileWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>, initial_sector_index: u16, file_id: u64) -> Self {
        FileWriter {
            vlfs,
            buffer: [0xFFu8; 5 + PAGE_SIZE],
            buffer_offset: 5,
            sector_data_length: 0,
            current_sector_index: initial_sector_index,
            file_id,
            closed: false,
        }
    }

    async fn write_page(
        &mut self,
        address: u32,
        crc_offset: Option<usize>,
    ) -> Result<(), VLFSError<F::Error>> {
        if let Some(crc_offset) = crc_offset {
            let mut crc = self.vlfs.crc.lock().await;
            let crc = crc.calculate(&self.buffer[5..(crc_offset + 5)]);
            (&mut self.buffer[(crc_offset + 5)..(crc_offset + 5 + 4)])
                .copy_from_slice(&crc.to_be_bytes());
        }

        let mut flash = self.vlfs.flash.lock().await;
        flash
            .write_256b(address, &mut self.buffer)
            .await
            .map_err(VLFSError::FlashError)?;

        self.buffer = [0xFFu8; 5 + 256];
        self.buffer_offset = 5;
        Ok(())
    }

    fn is_last_data_page(&self) -> bool {
        self.sector_data_length > (252 * 15)
    }

    fn page_address(&self) -> u32 {
        let current_sector_address = (self.current_sector_index as usize * SECTOR_SIZE) as u32;
        current_sector_address + ((self.sector_data_length as u32 - 1) & !255)
    }

    fn write_length_and_next_sector_index(&mut self, next_sector_index: u16) {
        let data_length_offset = self.buffer.len() - 8 - 8;
        (&mut self.buffer[data_length_offset..(data_length_offset + 8)])
            .copy_from_u16x4(self.sector_data_length);

        let next_sector_index_length_offset = self.buffer.len() - 8;
        (&mut self.buffer[next_sector_index_length_offset..(next_sector_index_length_offset + 8)])
            .copy_from_u16x4(next_sector_index);
    }

    pub async fn flush(&mut self) -> Result<(), VLFSError<F::Error>> {
        if self.sector_data_length == 0 {
            return Ok(());
        }

        let next_sector_index = self.vlfs.claim_avaliable_sector_and_erase().await?;
        self._flush(next_sector_index).await?;
        self.current_sector_index = next_sector_index;

        Ok(())
    }

    async fn _flush(&mut self, next_sector_index: u16) -> Result<(), VLFSError<F::Error>> {
        if self.sector_data_length == 0 {
            self.write_length_and_next_sector_index(next_sector_index);
            let current_sector_address = (self.current_sector_index as usize * SECTOR_SIZE) as u32;
            self.write_page(
                current_sector_address + (SECTOR_SIZE - PAGE_SIZE) as u32,
                None,
            )
            .await?;
            return Ok(());
        }

        // pad to 4 bytes
        let crc_offset = ((self.buffer_offset - 5) + 3) & !3;

        if self.is_last_data_page() {
            // last data page contains data
            self.write_length_and_next_sector_index(next_sector_index);

            self.write_page(self.page_address(), Some(crc_offset))
                .await?;
        } else {
            // last data page does not contain data
            self.write_page(self.page_address(), Some(crc_offset))
                .await?;

            self.write_length_and_next_sector_index(next_sector_index);

            let current_sector_address = (self.current_sector_index as usize * SECTOR_SIZE) as u32;
            self.write_page(
                current_sector_address + (SECTOR_SIZE - PAGE_SIZE) as u32,
                None,
            )
            .await?;
        }

        self.sector_data_length = 0;
        Ok(())
    }

    pub async fn close(mut self) -> Result<(), VLFSError<F::Error>> {
        // will cause an empty sector to be saved, when self.sector_data_length == 0,
        // alternative is to read-modify-write the previous sector.
        // shouldn't happen a lot in real world use cases, ignore for now
        self._flush(0xFFFF).await?;

        let mut at = self.vlfs.allocation_table.write().await;
        let file_entry = self
            .vlfs
            .find_file_entry_mut(&mut at.allocation_table, self.file_id)
            .unwrap();
        file_entry.opened = false;

        self.closed = true;
        Ok(())
    }
}

impl<'a, F, C> AsyncWriter for FileWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Error = VLFSError<F::Error>;

    async fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), VLFSError<F::Error>> {
        let mut slice = slice;
        while slice.len() > 0 {
            let buffer_reserved_size = if self.is_last_data_page() {
                8 + 8 + 4
            } else {
                4
            };
            let buffer_len = self.buffer.len();
            let buffer_free = self.buffer.len() - self.buffer_offset - buffer_reserved_size;

            if slice.len() < buffer_free {
                // if the slice fits inside available buffer space
                // and there are empty buffer space left after copying the slice

                (&mut self.buffer[self.buffer_offset..(self.buffer_offset + slice.len())])
                    .copy_from_slice(slice);
                self.buffer_offset += slice.len();
                self.sector_data_length += slice.len() as u16;

                slice = &[];
            } else {
                // 1. if slice fits inside available buffer space but there are no empty buffer space left after copying the slice
                // 2. if slice does not fit inside available buffer space

                (&mut self.buffer[self.buffer_offset..(buffer_len - buffer_reserved_size)])
                    .copy_from_slice(&slice[..buffer_free]);
                self.buffer_offset += buffer_free;
                self.sector_data_length += buffer_free as u16;

                if self.is_last_data_page() {
                    let next_sector_index = self.vlfs.claim_avaliable_sector_and_erase().await?;
                    self.write_length_and_next_sector_index(next_sector_index);

                    self.write_page(self.page_address(), Some(self.buffer_offset - 5))
                        .await?;

                    self.sector_data_length = 0;
                    self.current_sector_index = next_sector_index
                } else {
                    self.write_page(self.page_address(), Some(self.buffer_offset - 5))
                        .await?;
                }

                slice = &slice[buffer_free..];
            }
        }

        Ok(())
    }
}

impl<'a, F, C> Drop for FileWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn drop(&mut self) {
        if !self.closed {
            defmt::panic!(
                "FileWriter for file {:X} dropped without being closed",
                self.file_id
            );
        }
    }
}

impl<'a, F, C> fmt::Debug for FileWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileWriter")
            .field("file_id", &self.file_id)
            .field("sector_data_length", &self.sector_data_length)
            .field("current_sector_index", &self.current_sector_index)
            .finish()
    }
}
