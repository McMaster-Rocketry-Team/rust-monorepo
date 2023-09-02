use super::*;

// only repersent the state of the file when the struct is created
// does not update after that
#[derive(Debug, Clone, defmt::Format)]
pub struct FileEntry {
    pub opened: bool,
    pub file_id: FileID,
    pub file_type: FileType,
    pub(super) first_sector_index: Option<u16>, // None means the file is empty
}

impl FileEntry {
    pub(crate) fn new(file_id: FileID, file_type: FileType) -> Self {
        Self {
            opened: false,
            file_id,
            file_type,
            first_sector_index: None,
        }
    }
}

// serialized size must fit in half a block (32kib)
pub(super) struct AllocationTable {
    pub(super) sequence_number: u32,
    pub(super) allocation_table_position: usize, // which half block is the allocation table in
    pub(super) file_count: u16,
    pub(super) max_file_id: FileID,
    pub(super) opened_files: Vec<FileID, 10>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            allocation_table_position: 0,
            file_count: 0,
            max_file_id: FileID(0),
            opened_files: Vec::new(),
        }
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
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        defmt::todo!()
    }

    pub(super) async fn delete_file_entry(
        &self,
        file_id: FileID,
    ) -> Result<(), VLFSError<F::Error>> {
        defmt::todo!()
    }

    pub(super) async fn set_file_first_sector_index(
        &self,
        file_id: FileID,
        first_sector_index: u16,
    ) -> Result<(), VLFSError<F::Error>> {
        defmt::todo!()
    }

    pub(super) async fn write_new_file_entry(
        &self,
        file_entry: FileEntry,
    ) -> Result<(), VLFSError<F::Error>> {
        defmt::todo!()
    }

    // return true: found a valid allocation table
    pub(super) async fn read_latest_allocation_table(&self) -> Result<bool, VLFSError<F::Error>> {
        defmt::todo!()
    }

    pub(super) async fn write_empty_allocation_table(&self) -> Result<(), VLFSError<F::Error>> {
        defmt::todo!()
    }
}
