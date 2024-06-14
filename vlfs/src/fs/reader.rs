use core::fmt;

use crate::utils::io_traits::AsyncReader;
use embedded_io_async::{ErrorType, Read};

use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn open_file_for_read(
        &self,
        file_id: FileID,
    ) -> Result<FileReader<F, C>, VLFSError<F::Error>> {
        if let Some((file_entry, _)) = self.find_file_entry(file_id).await? {
            log_info!(
                "Opening file {:?} with id {:?} for read",
                file_id,
                file_entry.typ
            );
            if self.is_file_opened(file_entry.id).await {
                return Err(VLFSError::FileInUse);
            }

            self.mark_file_opened(file_id).await?;

            return Ok(FileReader::new(
                self,
                file_entry.first_sector_index,
                file_id,
            ));
        }
        Err(VLFSError::FileDoesNotExist)
    }
}

enum SectorDataLength {
    NotRead,   // Sector data length has not been read yet
    Read(u16), // Sector data length has been read
    Unknown,   // Sector data length has been read, but the value is invalid
}

#[derive(defmt::Format, Debug, Clone)]
pub enum VLFSReadStatus {
    Ok,
    EndOfFile,
    CorruptedPage { address: u32 },
}

pub struct FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    current_sector_index: Option<u16>,
    current_page_index: u16,
    sector_data_length: SectorDataLength,

    sector_read_data_length: u16,
    file_id: FileID,

    page_buffer: [u8; 5 + PAGE_SIZE],
    page_buffer_read_ahead_range: (usize, usize),

    closed: bool,
}

impl<'a, F, C> FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>, first_sector_index: Option<u16>, file_id: FileID) -> Self {
        Self {
            vlfs,
            sector_data_length: SectorDataLength::NotRead,
            sector_read_data_length: 0,
            current_sector_index: first_sector_index,
            current_page_index: 0,
            file_id: file_id,
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
        self.sector_data_length = SectorDataLength::NotRead;
        self.sector_read_data_length = 0;
    }

    /// read_next_page does not garantee that page_buffer is filled with file content, multiple calls may be needed
    async fn read_next_page(&mut self) -> Result<VLFSReadStatus, VLFSError<F::Error>> {
        if let Some(current_sector_index) = self.current_sector_index {
            let is_last_page = self.current_page_index == 15;
            let flash = self.vlfs.flash.read().await;
            let sector_address = current_sector_index as usize * SECTOR_SIZE;

            if let SectorDataLength::NotRead = self.sector_data_length {
                log_assert!(self.current_page_index == 0);
                let sector_data_length_address = (sector_address + SECTOR_SIZE - 8 - 8) as u32;
                let read_result = flash
                    .read(sector_data_length_address, 8, &mut self.page_buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;
                let sector_data_length = find_most_common_u16_out_of_4(read_result);
                if let Some(sector_data_length) = sector_data_length
                    && sector_data_length <= MAX_DATA_LENGTH_PER_SECTION as u16
                {
                    self.sector_data_length = SectorDataLength::Read(sector_data_length);
                } else {
                    self.sector_data_length = SectorDataLength::Unknown;
                }
            }

            let sector_unread_data_length =
                if let SectorDataLength::Read(sector_data_length) = self.sector_data_length {
                    Some(sector_data_length - self.sector_read_data_length)
                } else {
                    None
                };

            if let Some(sector_unread_data_length) = sector_unread_data_length {
                // Jump to next sector
                // This if statement is true when the sector is fully read before the last page
                if sector_unread_data_length == 0 {
                    let next_sector_index_address = (sector_address + SECTOR_SIZE - 8) as u32;
                    let read_result = flash
                        .read(next_sector_index_address, 8, &mut self.page_buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;

                    let next_sector_index = find_most_common_u16_out_of_4(read_result).unwrap();
                    self.set_current_sector_index(next_sector_index);
                    self.page_buffer_read_ahead_range = (0, 0);
                    return Ok(VLFSReadStatus::Ok);
                }
            }

            // Calculate how much data this page contains
            let max_data_length_in_page = if is_last_page {
                MAX_DATA_LENGTH_LAST_PAGE
            } else {
                MAX_DATA_LENGTH_PER_PAGE
            } as u16;
            let read_data_length =
                if let Some(sector_unread_data_length) = sector_unread_data_length {
                    core::cmp::min(sector_unread_data_length, max_data_length_in_page) as usize
                } else {
                    max_data_length_in_page as usize
                };

            // Calculate padding
            let read_data_length_padded = (read_data_length + 3) & !3;

            let page_address =
                (sector_address + self.current_page_index as usize * PAGE_SIZE) as u32;

            let read_result = flash
                .read(
                    page_address,
                    if is_last_page {
                        // Always read the full page if it is the last page
                        // This is because the next sector index information is stored at the end
                        256
                    } else {
                        // Only read the data and crc if it is not the last page
                        read_data_length_padded + 4
                    },
                    &mut self.page_buffer,
                )
                .await
                .map_err(VLFSError::FlashError)?;
            self.sector_read_data_length += read_data_length as u16;
            drop(flash);

            // Check CRC
            let data_buffer_padded = &read_result[..read_data_length_padded];
            let expected_crc_buffer =
                &read_result[read_data_length_padded..(read_data_length_padded + 4)];
            let expected_crc = u32::from_be_bytes(expected_crc_buffer.try_into().unwrap());
            let mut crc = self.vlfs.crc.lock().await;
            let actual_crc = crc.calculate(data_buffer_padded);
            drop(crc);

            if is_last_page {
                let next_sector_index =
                    find_most_common_u16_out_of_4(&read_result[(PAGE_SIZE - 8)..]).unwrap();
                self.set_current_sector_index(next_sector_index);
            } else {
                self.current_page_index += 1;
            }

            if actual_crc != expected_crc {
                log_info!(
                    "CRC mismatch: expected {}, actual {}",
                    expected_crc,
                    actual_crc
                );
                // Tell the application the page is corrupted
                // and let the application decide if it wants to continue reading
                self.page_buffer_read_ahead_range = (0, 0);
                return Ok(VLFSReadStatus::CorruptedPage {
                    address: page_address,
                });
            } else {
                self.page_buffer_read_ahead_range = (5, 5 + read_data_length);
                return Ok(VLFSReadStatus::Ok);
            }
        } else {
            self.page_buffer_read_ahead_range = (0, 0);
            return Ok(VLFSReadStatus::EndOfFile);
        }
    }

    pub async fn close(mut self) {
        log_info!("Closing file with id {:?} for read", self.file_id,);
        self.vlfs.mark_file_closed(self.file_id).await;
        self.closed = true;
    }
}

impl<'a, F, C> AsyncReader for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Error = VLFSError<F::Error>;
    type ReadStatus = VLFSReadStatus;

    async fn read_slice<'b>(
        &mut self,
        read_buffer: &'b mut [u8],
        length: usize,
    ) -> Result<(&'b [u8], VLFSReadStatus), VLFSError<F::Error>> {
        let mut read_length = 0;

        while read_length < length {
            let read_ahead_slice = &self.page_buffer
                [self.page_buffer_read_ahead_range.0..self.page_buffer_read_ahead_range.1];
            if read_ahead_slice.len() == 0 {
                match self.read_next_page().await {
                    Ok(VLFSReadStatus::Ok) => {
                        continue;
                    }
                    Ok(VLFSReadStatus::EndOfFile) => {
                        return Ok((&read_buffer[..read_length], VLFSReadStatus::EndOfFile));
                    }
                    Ok(VLFSReadStatus::CorruptedPage { address }) => {
                        return Ok((
                            &read_buffer[..read_length],
                            VLFSReadStatus::CorruptedPage { address },
                        ));
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

        Ok((&read_buffer[..read_length], VLFSReadStatus::Ok))
    }
}

impl<'a, F, C> ErrorType for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Error = VLFSError<F::Error>;
}

impl<'a, F, C> Read for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // TODO refactor VLFS to use embedded-io-async internally
        self.read_slice(buf, buf.len())
            .await
            .map(|(buffer, _)| buffer.len())
    }
}

impl<'a, F, C> Drop for FileReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fn drop(&mut self) {
        if !self.closed {
            log_panic!(
                "FileReader for file {:X} dropped without being closed",
                self.file_id.0
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
