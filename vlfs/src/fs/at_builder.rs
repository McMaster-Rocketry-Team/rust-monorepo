use embassy_sync::blocking_mutex::raw::NoopRawMutex;

use crate::{
    flash::flash_wrapper::FlashWrapper, utils::rwlock::RwLockWriteGuard, Crc, FileEntry,
    FileWriter, Flash, VLFSError,
};

use super::{
    allocation_table::{
        AllocationTable, AllocationTableFooter, ALLOC_TABLE_HEADER_SIZE, FILE_ENTRY_SIZE,
    },
    sector_management::SectorsMng,
    utils::find_most_common_u16_out_of_4,
    FileID, FileType, SECTOR_SIZE, VLFS,
};

const READ_FILE_ENTRY_BATCH_SIZE: usize = 64;

/// This struct does not check if the new allocation table is valid.
/// All the files entries should have increasing file ids.
/// It is possible to create more file entries than fit in the allocation table.
/// It is possible to delete opened files.
pub struct ATBuilder<'a, 'b, F: Flash, C: Crc>
where
    'b: 'a,
{
    at: RwLockWriteGuard<'a, NoopRawMutex, AllocationTable, 10>,
    flash: RwLockWriteGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,
    sectors_mng: RwLockWriteGuard<'a, NoopRawMutex, SectorsMng, 10>,
    fs: &'b VLFS<F, C>,

    curr_at_start_addr: u32,
    read_buffer: [u8; 5 + FILE_ENTRY_SIZE * READ_FILE_ENTRY_BATCH_SIZE],
    read_file_entry_i: usize,
    read_finished: bool,

    write_page_address: u32,
    write_buffer: [u8; 5 + 256],
    write_buffer_offset: usize,

    pub(super) file_count: u16,
    pub(super) max_file_id: FileID,

    finished: bool,
}

impl<'a, 'b, F: Flash, C: Crc> ATBuilder<'a, 'b, F, C>
where
    'b: 'a,
{
    pub(crate) async fn new(fs: &'b VLFS<F, C>) -> Result<Self, VLFSError<F::Error>> {
        let mut at = fs.allocation_table.write().await;
        let max_file_id = at.footer.max_file_id;
        let curr_at_address = at.address();
        at.increment_position();
        let new_at_address = at.address();

        let flash = fs.flash.write().await;
        let sectors_mng = fs.sectors_mng.write().await;

        let mut builder = Self {
            at,
            flash,
            sectors_mng,
            fs,

            curr_at_start_addr: curr_at_address,
            read_buffer: [0u8; 5 + FILE_ENTRY_SIZE * READ_FILE_ENTRY_BATCH_SIZE],
            read_file_entry_i: 0,
            read_finished: false,

            write_page_address: new_at_address,
            write_buffer: [0xFF; 5 + 256],
            write_buffer_offset: 5,

            file_count: 0,
            max_file_id,

            finished: false,
        };

        builder.erase_and_write_header().await?;
        Ok(builder)
    }

    async fn read_next_file_entry_slice(&mut self) -> Result<&[u8], F::Error> {
        if self.read_file_entry_i % READ_FILE_ENTRY_BATCH_SIZE == 0 {
            let read_address = self.curr_at_start_addr
                + ALLOC_TABLE_HEADER_SIZE as u32
                + self.read_file_entry_i as u32 * FILE_ENTRY_SIZE as u32;
            self.flash
                .read(
                    read_address,
                    FILE_ENTRY_SIZE * READ_FILE_ENTRY_BATCH_SIZE,
                    &mut self.read_buffer,
                )
                .await?;
        }

        let start = (self.read_file_entry_i % READ_FILE_ENTRY_BATCH_SIZE) * FILE_ENTRY_SIZE + 5;
        let end = start + FILE_ENTRY_SIZE;
        self.read_file_entry_i += 1;
        return Ok(&self.read_buffer[start..end]);
    }

    /// Read the next file entry in the current allocation table.
    /// Returns None if the end of the allocation table is reached.
    pub async fn read_next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        if self.read_finished {
            return Ok(None);
        }
        let read_result = self
            .read_next_file_entry_slice()
            .await
            .map_err(VLFSError::FlashError)?;
        if let Ok(file_entry) = FileEntry::deserialize(read_result) {
            if AllocationTableFooter::is_footer_file_entry(&file_entry) {
                log_info!("found footer entry");
                self.read_finished = true;
                return Ok(None);
            } else {
                log_info!("found file entry");
                return Ok(Some(file_entry));
            }
        } else {
            return Err(VLFSError::CorruptedFileEntry);
        }
    }

    async fn flush(&mut self) -> Result<(), F::Error> {
        self.flash
            .write_256b(self.write_page_address, &mut self.write_buffer)
            .await?;
        self.write_page_address += 256;
        self.write_buffer = [0xFF; 5 + 256];
        self.write_buffer_offset = 5;
        Ok(())
    }

    async fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), F::Error> {
        let mut slice = slice;
        while slice.len() > 0 {
            let buffer_free = self.write_buffer.len() - self.write_buffer_offset;

            if slice.len() < buffer_free {
                (&mut self.write_buffer
                    [self.write_buffer_offset..(self.write_buffer_offset + slice.len())])
                    .copy_from_slice(slice);
                self.write_buffer_offset += slice.len();

                slice = &[];
            } else {
                (&mut self.write_buffer[self.write_buffer_offset..])
                    .copy_from_slice(&slice[..buffer_free]);
                self.write_buffer_offset += buffer_free;

                self.flush().await?;

                slice = &slice[buffer_free..];
            }
        }

        Ok(())
    }

    /// Write a file entry to the new allocation table.
    pub async fn write(&mut self, file_entry: &FileEntry) -> Result<(), VLFSError<F::Error>> {
        self.extend_from_slice(&file_entry.serialize())
            .await
            .map_err(VLFSError::FlashError)?;
        self.file_count += 1;
        if file_entry.id > self.max_file_id {
            self.max_file_id = file_entry.id
        }
        Ok(())
    }

    pub async fn write_new_file(
        &mut self,
        file_type: FileType,
    ) -> Result<FileEntry, VLFSError<F::Error>> {
        let file_entry = FileEntry::new(self.get_new_file_id(), file_type);
        self.write(&file_entry).await?;
        Ok(file_entry)
    }

    pub async fn write_new_file_and_open_for_write(
        &mut self,
        file_type: FileType,
    ) -> Result<FileWriter<'b, F, C>, VLFSError<F::Error>> {
        log_trace!("write_new_file_and_open_for_write");
        let mut file_entry = FileEntry::new(self.get_new_file_id(), file_type);
        let new_sector_index = self.sectors_mng.claim_avaliable_sector_and_erase(&mut self.flash).await?;
        file_entry.first_sector_index = Some(new_sector_index);
        self.write(&file_entry).await?;

        Ok(FileWriter::new(self.fs, new_sector_index, file_entry.id))
    }

    pub fn get_new_file_id(&mut self) -> FileID {
        self.max_file_id.0 += 1;
        self.max_file_id
    }

    async fn erase_and_write_header(&mut self) -> Result<(), VLFSError<F::Error>> {
        self.flash
            .erase_block_32kib(self.write_page_address)
            .await
            .map_err(VLFSError::FlashError)?;
        self.extend_from_slice(&self.at.header.serialize())
            .await
            .map_err(VLFSError::FlashError)?;
        Ok(())
    }

    pub fn is_file_opened(&self, file_id: FileID) -> bool {
        self.at.opened_files.iter().any(|&id| id == file_id)
    }

    /// If you remove a file entry from the allocation table, you must also
    /// release the sectors used by the file. Or the space occupied by the file
    /// won't be available until next reboot.
    pub async fn release_file_sectors(
        &mut self,
        file_entry: &FileEntry,
    ) -> Result<(), VLFSError<F::Error>> {
        let mut current_sector_index = file_entry.first_sector_index;
        let mut buffer = [0u8; 5 + 8];
        while let Some(sector_index) = current_sector_index {
            let address = sector_index as u32 * SECTOR_SIZE as u32;
            let address = address + SECTOR_SIZE as u32 - 8;

            let read_result = self
                .flash
                .read(address, 8, &mut buffer)
                .await
                .map_err(VLFSError::FlashError)?;
            let next_sector_index = find_most_common_u16_out_of_4(read_result).unwrap();
            self.sectors_mng.sector_map.set_sector_unused(sector_index);
            current_sector_index = if next_sector_index == 0xFFFF {
                None
            } else {
                Some(next_sector_index)
            };
        }

        Ok(())
    }

    pub async fn commit(mut self) -> Result<(), VLFSError<F::Error>> {
        log_info!("commit");
        self.at.footer.file_count = self.file_count;
        self.at.footer.max_file_id = self.max_file_id;
        self.extend_from_slice(&self.at.footer.serialize())
            .await
            .map_err(VLFSError::FlashError)?;
        self.flush().await.map_err(VLFSError::FlashError)?;
        self.finished = true;
        Ok(())
    }
}

impl<'a, 'b, F: Flash, C: Crc> Drop for ATBuilder<'a, 'b, F, C>
where
    'b: 'a,
{
    fn drop(&mut self) {
        if !self.finished {
            log_panic!("FATBuilder dropped without calling commit()");
        }
    }
}
