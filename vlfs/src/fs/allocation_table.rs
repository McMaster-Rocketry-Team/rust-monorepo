use core::future::Future;

use crate::{
    utils::flash_io::{FlashReader, FlashWriter},
    AsyncReader, AsyncWriter, DummyCrc,
};

use super::{
    hamming::{hamming_decode, hamming_encode},
    *,
};

const ALLOC_TABLE_HEADER_SIZE: usize = 13;
pub const FILE_ENTRY_SIZE: usize = 13;

// only repersent the state of the file when the struct is created
// does not update after that
#[derive(Debug, Clone, defmt::Format, PartialEq, Eq)]
pub struct FileEntry {
    pub id: FileID,
    pub typ: FileType,
    pub(super) first_sector_index: Option<u16>, // None means the file is empty
}

pub(crate) struct CorruptedFileEntry;

impl FileEntry {
    pub(crate) fn new(file_id: FileID, file_type: FileType) -> Self {
        Self {
            id: file_id,
            typ: file_type,
            first_sector_index: None,
        }
    }

    pub(crate) fn serialize(&self) -> [u8; FILE_ENTRY_SIZE] {
        let mut buffer = [0u8; FILE_ENTRY_SIZE];
        (&mut buffer[0..2]).copy_from_slice(&self.typ.0.to_be_bytes());
        if let Some(first_sector_index) = self.first_sector_index {
            (&mut buffer[2..4]).copy_from_slice(&first_sector_index.to_be_bytes());
        } else {
            (&mut buffer[2..4]).copy_from_slice(&0xFFFFu16.to_be_bytes());
        }
        (&mut buffer[4..12]).copy_from_slice(&self.id.0.to_be_bytes());

        hamming_encode(buffer)
    }

    // expect a FILE_ENTRY_SIZE byte buffer
    pub(crate) fn deserialize(buffer: &[u8]) -> Result<Self, CorruptedFileEntry> {
        log_trace!("Deserializing file entry: {:?}", buffer);
        let buffer = hamming_decode(buffer.try_into().unwrap()).map_err(|_| CorruptedFileEntry)?;

        let file_type = FileType(u16::from_be_bytes((&buffer[0..2]).try_into().unwrap()));
        let first_sector_index = u16::from_be_bytes((&buffer[2..4]).try_into().unwrap());
        let file_id = FileID(u64::from_be_bytes((&buffer[4..12]).try_into().unwrap()));
        Ok(Self {
            id: file_id,
            typ: file_type,
            first_sector_index: if first_sector_index == 0xFFFF {
                None
            } else {
                Some(first_sector_index)
            },
        })
    }
}

pub(crate) struct CorruptedAllocationTableHeader;

pub(super) struct AllocationTableHeader {
    pub(super) sequence_number: u64,
}

impl Default for AllocationTableHeader {
    fn default() -> Self {
        Self { sequence_number: 1 }
    }
}

impl AllocationTableHeader {
    pub(crate) fn serialize(&self) -> [u8; ALLOC_TABLE_HEADER_SIZE] {
        let mut buffer = [0u8; 13];
        (&mut buffer[0..4]).copy_from_slice(&VLFS_VERSION.to_be_bytes());
        (&mut buffer[4..12]).copy_from_slice(&self.sequence_number.to_be_bytes());

        hamming_encode(buffer)
    }

    // expect a ALLOC_TABLE_HEADER_SIZE byte buffer
    pub(crate) fn deserialize(buffer: &[u8]) -> Result<Self, CorruptedAllocationTableHeader> {
        let buffer1 = hamming_decode(buffer[0..13].try_into().unwrap())
            .map_err(|_| CorruptedAllocationTableHeader)?;

        let version = u32::from_be_bytes((&buffer1[0..4]).try_into().unwrap());
        let sequence_number = u64::from_be_bytes((&buffer1[4..12]).try_into().unwrap());

        if version != VLFS_VERSION {
            log_warn!(
                "Version mismatch, expected: {}, actual: {}",
                VLFS_VERSION,
                version
            );
            return Err(CorruptedAllocationTableHeader);
        }

        Ok(Self { sequence_number })
    }
}

pub(super) struct AllocationTableFooter {
    pub(super) file_count: u16,
    pub(super) max_file_id: FileID,
}

impl Default for AllocationTableFooter {
    fn default() -> Self {
        Self {
            file_count: 0,
            max_file_id: FileID(0),
        }
    }
}

/// allocation table footer stores data as a file entry with reserved type (0xFFFF)
impl AllocationTableFooter {
    pub(crate) fn serialize(&self) -> [u8; FILE_ENTRY_SIZE] {
        FileEntry {
            id: self.max_file_id,
            typ: FileType(0xFFFF),
            first_sector_index: None,
        }
        .serialize()
    }

    pub(crate) fn is_footer_file_entry(file_entry: &FileEntry) -> bool {
        file_entry.typ == FileType(0xFFFF)
    }

    pub(crate) fn deserialize(file_count: u16, file_entry: &FileEntry) -> Self {
        Self {
            file_count,
            max_file_id: file_entry.id,
        }
    }
}

// serialized size must fit in half a block (32kib)
pub(super) struct AllocationTable {
    pub(super) header: AllocationTableHeader,
    pub(super) footer: AllocationTableFooter,
    pub(super) allocation_table_position: usize, // which half block is the allocation table in
    pub(super) opened_files: Vec<FileID, 32>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            header: AllocationTableHeader::default(),
            footer: AllocationTableFooter::default(),
            allocation_table_position: 0,
            opened_files: Vec::new(),
        }
    }
}

impl AllocationTable {
    pub(super) fn address(&self) -> u32 {
        (self.allocation_table_position * 32 * 1024) as u32
    }

    // does not garuntee that the file entry is valid
    pub(super) fn address_of_file_entry(&self, i: u16) -> u32 {
        self.address() + ALLOC_TABLE_HEADER_SIZE as u32 + (i as u32) * FILE_ENTRY_SIZE as u32
    }

    pub(super) fn increment_position(&mut self) {
        self.allocation_table_position = (self.allocation_table_position + 1) % TABLE_COUNT;
        self.header.sequence_number += 1;
    }
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    // WARNING: this function does not check if the file is already opened
    pub(super) async fn mark_file_opened(
        &self,
        file_id: FileID,
    ) -> Result<(), VLFSError<F::Error>> {
        let mut at = self.allocation_table.write().await;
        at.opened_files
            .push(file_id)
            .map_err(|_| VLFSError::TooManyFilesOpen)?;
        Ok(())
    }

    pub async fn is_file_opened(&self, file_id: FileID) -> bool {
        let at = self.allocation_table.read().await;
        at.opened_files.iter().any(|&id| id == file_id)
    }

    pub(super) async fn mark_file_closed(&self, file_id: FileID) {
        let mut at = self.allocation_table.write().await;
        at.opened_files
            .iter()
            .position(|&id| id == file_id)
            .map(|index| {
                at.opened_files.swap_remove(index);
            });
    }

    pub(super) async fn find_file_entry(
        &self,
        file_id: FileID,
    ) -> Result<Option<(FileEntry, u16)>, VLFSError<F::Error>> {
        let flash = self.flash.read().await;
        let at = self.allocation_table.read().await;
        let file_count = at.footer.file_count;
        if file_count == 0 {
            return Ok(None);
        }

        let mut buffer = [0u8; 5 + FILE_ENTRY_SIZE];
        let mut dummy_crc = DummyCrc {};
        let mut reader = FlashReader::new(0, &flash, &mut dummy_crc);
        let mut left = 0;
        let mut right = file_count - 1;

        while left < right {
            let mid = (left + right) / 2;
            reader.set_address(at.address_of_file_entry(mid));
            let (read_result, _) = reader
                .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                .await
                .map_err(VLFSError::FlashError)?;
            let file_entry = FileEntry::deserialize(read_result)?;
            if file_entry.id < file_id {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        reader.set_address(at.address_of_file_entry(left));
        let (read_result, _) = reader
            .read_slice(&mut buffer, FILE_ENTRY_SIZE)
            .await
            .map_err(VLFSError::FlashError)?;
        let file_entry = FileEntry::deserialize(read_result)?;
        if file_entry.id == file_id {
            return Ok(Some((file_entry, left)));
        } else {
            return Ok(None);
        }
    }

    // FIXME
    pub(super) async fn delete_file_entry(
        &self,
        file_id: FileID,
    ) -> Result<(), VLFSError<F::Error>> {
        if let Some((_, file_entry_i)) = self.find_file_entry(file_id).await? {
            let mut at = self.allocation_table.write().await;
            let old_at_address = at.address();
            at.increment_position();
            let at_address = at.address();

            let mut flash = self.flash.write().await;
            flash
                .erase_block_32kib(at_address)
                .await
                .map_err(VLFSError::FlashError)?;
            drop(flash);

            let mut crc = self.crc.lock().await;
            let mut dummy_crc = DummyCrc {};
            let mut reader_flash = &self.flash;
            let mut writer_flash = &self.flash;
            let mut reader = FlashReader::new(
                old_at_address + ALLOC_TABLE_HEADER_SIZE as u32,
                &mut reader_flash,
                &mut dummy_crc,
            );
            let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

            // write header to the new allocation table
            writer
                .extend_from_slice(&at.header.serialize())
                .await
                .map_err(VLFSError::FlashError)?;

            // copy entries before the deleted entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            let mut buffer = [0u8; 5 + FILE_ENTRY_SIZE];
            for _ in 0..file_entry_i {
                let (result, _) = reader
                    .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(result)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            reader.set_address(reader.get_address() + FILE_ENTRY_SIZE as u32);

            // copy entries after the deleted entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            for _ in file_entry_i + 1..at.footer.file_count {
                let (result, _) = reader
                    .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(result)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write crc
            writer
                .extend_from_u32(writer.get_crc())
                .await
                .map_err(VLFSError::FlashError)?;

            writer.flush().await.map_err(VLFSError::FlashError)?;

            at.footer.file_count -= 1;
            return Ok(());
        } else {
            return Err(VLFSError::FileDoesNotExist);
        }
    }

    // FIXME
    pub(super) async fn set_file_first_sector_index(
        &self,
        file_id: FileID,
        first_sector_index: Option<u16>,
    ) -> Result<(), VLFSError<F::Error>> {
        if let Some((mut file_entry, file_entry_i)) = self.find_file_entry(file_id).await? {
            let mut at = self.allocation_table.write().await;
            let old_at_address = at.address();
            at.increment_position();
            let at_address = at.address();

            let mut flash = self.flash.write().await;
            flash
                .erase_block_32kib(at_address)
                .await
                .map_err(VLFSError::FlashError)?;
            drop(flash);

            let mut crc = self.crc.lock().await;
            let mut dummy_crc = DummyCrc {};
            let mut reader_flash = &self.flash;
            let mut writer_flash = &self.flash;
            let mut reader = FlashReader::new(
                old_at_address + ALLOC_TABLE_HEADER_SIZE as u32,
                &mut reader_flash,
                &mut dummy_crc,
            );
            let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

            // write header to the new allocation table
            writer
                .extend_from_slice(&at.header.serialize())
                .await
                .map_err(VLFSError::FlashError)?;

            // copy entries before the updated entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            let mut buffer = [0u8; 5 + FILE_ENTRY_SIZE];
            for _ in 0..file_entry_i {
                let (result, _) = reader
                    .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(result)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write updated file entry
            file_entry.first_sector_index = first_sector_index;
            writer
                .extend_from_slice(&file_entry.serialize())
                .await
                .map_err(VLFSError::FlashError)?;
            reader.set_address(reader.get_address() + FILE_ENTRY_SIZE as u32);

            // copy entries after the updated entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            for _ in file_entry_i + 1..at.footer.file_count {
                let (result, _) = reader
                    .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(result)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write crc
            writer
                .extend_from_u32(writer.get_crc())
                .await
                .map_err(VLFSError::FlashError)?;

            writer.flush().await.map_err(VLFSError::FlashError)?;

            return Ok(());
        } else {
            return Err(VLFSError::FileDoesNotExist);
        }
    }

    // FIXME
    pub async fn create_file(&self, file_type: FileType) -> Result<FileEntry, VLFSError<F::Error>> {
        log_trace!("Creating file with type: {:?}", file_type);
        let mut at = self.allocation_table.write().await;
        log_trace!("Aquired allocation table write lock");
        at.footer.max_file_id.increment();
        let old_at_address = at.address();
        at.increment_position();
        at.footer.file_count += 1;
        let at_address = at.address();
        let file_id = at.footer.max_file_id;

        let mut flash = self.flash.write().await;
        flash
            .erase_block_32kib(at_address)
            .await
            .map_err(VLFSError::FlashError)?;
        drop(flash);

        let mut crc = self.crc.lock().await;
        let mut dummy_crc = DummyCrc {};
        let mut reader_flash = &self.flash;
        let mut writer_flash = &self.flash;
        let mut reader = FlashReader::new(
            old_at_address + ALLOC_TABLE_HEADER_SIZE as u32,
            &mut reader_flash,
            &mut dummy_crc,
        );
        let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

        // write header to the new allocation table
        writer
            .extend_from_slice(&at.header.serialize())
            .await
            .map_err(VLFSError::FlashError)?;

        // copy existing file entries
        // TODO optimize this, we can read multiple file entries at once at the expense of more memory
        let mut buffer = [0u8; 5 + FILE_ENTRY_SIZE];
        for _ in 0..at.footer.file_count - 1 {
            let (result, _) = reader
                .read_slice(&mut buffer, FILE_ENTRY_SIZE)
                .await
                .map_err(VLFSError::FlashError)?;
            writer
                .extend_from_slice(result)
                .await
                .map_err(VLFSError::FlashError)?;
        }

        // write new file entry
        let file_entry = FileEntry::new(file_id, file_type);
        writer
            .extend_from_slice(&file_entry.serialize())
            .await
            .map_err(VLFSError::FlashError)?;

        // write crc
        writer
            .extend_from_u32(writer.get_crc())
            .await
            .map_err(VLFSError::FlashError)?;

        writer.flush().await.map_err(VLFSError::FlashError)?;

        log_info!("{:?} created, {:?}", &file_entry, file_entry.serialize());
        Ok(file_entry)
    }

    // return true: found a valid allocation table
    pub(super) async fn read_latest_allocation_table(&self) -> Result<bool, VLFSError<F::Error>> {
        let mut found_valid_table = false;
        let mut flash = self.flash.write().await;
        let mut crc = DummyCrc {};

        'outer: for i in 0..TABLE_COUNT {
            log_info!("Reading allocation table #{}", i + 1);

            let mut read_buffer = [0u8; 5 + ALLOC_TABLE_HEADER_SIZE];
            let mut reader =
                FlashReader::new((i * 32 * 1024).try_into().unwrap(), &mut flash, &mut crc);

            let read_result = reader
                .read_slice(&mut read_buffer, ALLOC_TABLE_HEADER_SIZE)
                .await
                .map_err(VLFSError::FlashError)?
                .0;

            let header = if let Ok(header) = AllocationTableHeader::deserialize(&read_result) {
                header
            } else {
                log_info!("Invalid header, skipping to next allocation table");
                continue 'outer;
            };

            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            let mut file_count = 0u16;
            let footer = loop {
                let (read_result, _) = reader
                    .read_slice(&mut read_buffer, FILE_ENTRY_SIZE)
                    .await
                    .map_err(VLFSError::FlashError)?;
                if let Ok(file_entry) = FileEntry::deserialize(read_result) {
                    if AllocationTableFooter::is_footer_file_entry(&file_entry) {
                        break AllocationTableFooter::deserialize(file_count, &file_entry);
                    } else {
                        file_count += 1;
                    }
                } else {
                    log_info!("Corrupted file entry, skipping to next allocation table");
                    continue 'outer;
                }
            };
            log_info!("Found {} files", file_count);

            let mut at = self.allocation_table.write().await;
            if header.sequence_number >= at.header.sequence_number {
                at.header = header;
                at.footer = footer;
                at.allocation_table_position = i;
                found_valid_table = true;
            }
        }

        return Ok(found_valid_table);
    }

    pub(super) async fn write_empty_allocation_table(&self) -> Result<(), VLFSError<F::Error>> {
        let at = self.allocation_table.read().await;
        let at_address = at.address();

        let mut flash = self.flash.write().await;
        flash
            .erase_sector_4kib(at_address)
            .await
            .map_err(VLFSError::FlashError)?;

        let mut crc = DummyCrc {};
        let mut writer = FlashWriter::new(at_address, &mut flash, &mut crc);

        writer
            .extend_from_slice(&at.header.serialize())
            .await
            .map_err(VLFSError::FlashError)?;

        writer
            .extend_from_slice(&at.footer.serialize())
            .await
            .map_err(VLFSError::FlashError)?;

        writer.flush().await.map_err(VLFSError::FlashError)?;

        Ok(())
    }

    pub async fn dangerously_modify_allocation_table<'a, FN, R, T>(
        &'a self,
        op: FN,
    ) -> Result<T, VLFSError<F::Error>>
    where
        FN: for<'b> FnOnce(
            &'b mut FileEntriesReader<'a, F, C>,
            &'b mut FileEntriesWriter<'a, F, C>,
        ) -> R,
        R: Future<Output = Result<T, VLFSError<F::Error>>>,
    {
        let mut at = self.allocation_table.write().await;
        let old_at_address = at.address();
        at.increment_position();
        let at_address = at.address();

        let mut flash = self.flash.write().await;
        flash
            .erase_block_32kib(at_address)
            .await
            .map_err(VLFSError::FlashError)?;
        drop(flash);

        let mut reader_flash = &self.flash;
        let mut writer_flash = &self.flash;

        let mut crc = self.crc.lock().await;
        let mut dummy_crc = DummyCrc {};

        let mut reader = FlashReader::new(
            old_at_address + ALLOC_TABLE_HEADER_SIZE as u32,
            &mut reader_flash,
            &mut dummy_crc,
        );
        let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

        let mut reader = FileEntriesReader { fs: self };
        let mut writer = FileEntriesWriter { fs: self };

        op(&mut reader, &mut writer).await?;

        todo!()
    }
}
pub struct FileEntriesReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fs: &'a VLFS<F, C>,
}

impl<'a, F, C> FileEntriesReader<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        todo!()
    }
}

pub struct FileEntriesWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    fs: &'a VLFS<F, C>,
}

impl<'a, F, C> FileEntriesWriter<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn write(
        &mut self,
        file_entry: &FileEntry,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        todo!()
    }
}
