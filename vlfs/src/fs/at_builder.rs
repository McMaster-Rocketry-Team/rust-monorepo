use embassy_sync::blocking_mutex::raw::NoopRawMutex;

use crate::{
    flash::flash_wrapper::FlashWrapper, utils::rwlock::RwLockWriteGuard, Crc, FileEntry, Flash,
    VLFSError,
};

use super::{
    allocation_table::{
        AllocationTable, AllocationTableFooter, ALLOC_TABLE_HEADER_SIZE, FILE_ENTRY_SIZE,
    }, FileID, FileType, VLFS
};

pub struct ATBuilder<'a, F: Flash> {
    at: RwLockWriteGuard<'a, NoopRawMutex, AllocationTable, 10>,
    flash: RwLockWriteGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,

    read_address: u32,
    read_buffer: [u8; 5 + FILE_ENTRY_SIZE],
    read_finished: bool,

    write_page_address: u32,
    write_buffer: [u8; 5 + 256],
    write_buffer_offset: usize,

    pub(super) file_count: u16,
    pub(super) max_file_id: FileID,

    finished: bool,
}

impl<'a, F: Flash> ATBuilder<'a, F> {
    pub(crate) async fn new(fs: &'a VLFS<F, impl Crc>) -> Result<Self, VLFSError<F::Error>> {
        let mut at = fs.allocation_table.write().await;
        let curr_at_address = at.address();
        at.increment_position();
        let new_at_address = at.address();

        let flash = fs.flash.write().await;

        let mut builder = Self {
            at,
            flash,
            read_address: curr_at_address + ALLOC_TABLE_HEADER_SIZE as u32,
            read_buffer: [0u8; 5 + FILE_ENTRY_SIZE],
            read_finished: false,

            write_page_address: new_at_address,
            write_buffer: [0xFF; 5 + 256],
            write_buffer_offset: 5,

            file_count: 0,
            max_file_id: FileID(0),

            finished: false,
        };

        builder.erase_and_write_header().await?;
        Ok(builder)
    }

    async fn read_slice(&mut self) -> Result<&[u8], F::Error> {
        self.flash
            .read(self.read_address, FILE_ENTRY_SIZE, &mut self.read_buffer)
            .await?;
        self.read_address += FILE_ENTRY_SIZE as u32;

        let read_result = &self.read_buffer[5..];
        Ok(read_result)
    }

    /// Read the next file entry in the current allocation table.
    /// Returns None if the end of the allocation table is reached.
    pub async fn read_next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        if self.read_finished {
            return Ok(None);
        }
        let read_result = self.read_slice().await.map_err(VLFSError::FlashError)?;
        if let Ok(file_entry) = FileEntry::deserialize(read_result) {
            if AllocationTableFooter::is_footer_file_entry(&file_entry) {
                self.read_finished = true;
                return Ok(None);
            } else {
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

    pub async fn write_new_file(&mut self, file_type: FileType) -> Result<FileEntry, VLFSError<F::Error>> {
        let file_entry = FileEntry::new(self.get_new_file_id(), file_type);
        self.write(&file_entry).await?;
        Ok(file_entry)
    }

    pub fn get_new_file_id(&mut self) -> FileID {
        self.max_file_id.0 +=1;
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

    pub async fn commit(mut self) -> Result<(), VLFSError<F::Error>> {
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

impl<'a, F: Flash> Drop for ATBuilder<'a, F> {
    fn drop(&mut self) {
        if !self.finished {
            log_panic!("FATBuilder dropped without calling commit()");
        }
    }
}
