use core::cmp::min;

use allocation_table::FILE_ENTRY_SIZE;

use crate::{
    utils::{flash_io::FlashReader, rwlock::RwLockReadGuard},
    AsyncReader, DummyCrc,
};

use super::*;

/// To allow all the files, use `()` as filter.
/// To allow only files of a certain type, use `FileType` as filter.
pub trait FileEntryFilter {
    fn check(&mut self, file_entry: &FileEntry) -> bool;
}

impl<P> FileEntryFilter for P
where
    P: FnMut(&FileEntry) -> bool,
{
    fn check(&mut self, file_entry: &FileEntry) -> bool {
        self(file_entry)
    }
}

impl FileEntryFilter for FileType {
    fn check(&mut self, file_entry: &FileEntry) -> bool {
        file_entry.typ == *self
    }
}

impl FileEntryFilter for Option<FileType> {
    fn check(&mut self, file_entry: &FileEntry) -> bool {
        if let Some(file_type) = self {
            file_entry.typ == *file_type
        } else {
            true
        }
    }
}

impl FileEntryFilter for () {
    fn check(&mut self, _: &FileEntry) -> bool {
        true
    }
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    /// Creating or deleting files while iterating over the files is NOT supported.
    /// The files returned by this iterator are guaranteed to be in increasing order of file id.
    /// If you need to create or delete files while iterating, use `concurrent_files_iter`.
    pub async fn files_iter<P: FileEntryFilter>(&self, filter: P) -> FilesIterator<F, P> {
        FilesIterator::new(&self, filter).await
    }

    /// Creating or deleting files while iterating over the files is supported.
    /// The files returned by this iterator are guaranteed to be in increasing order of file id.
    /// This is less efficient than `files_iter`.
    pub async fn concurrent_files_iter<P: FileEntryFilter>(
        &self,
        filter: P,
    ) -> ConcurrentFilesIterator<F, C, P> {
        ConcurrentFilesIterator::new(&self, filter).await
    }

    /// Find the first file (the file that has the smallest file id) that satisfies the predicate.
    pub async fn find_first_file<P: FileEntryFilter>(
        &self,
        filter: P,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        let mut iter = self.files_iter(filter).await;
        iter.next().await
    }
}

async fn read_file_entry<'a, F: Flash, const M: usize, const N: usize>(
    i: u16,
    at: &RwLockReadGuard<'a, NoopRawMutex, AllocationTable, M>,
    flash: &RwLockReadGuard<'a, NoopRawMutex, FlashWrapper<F>, N>,
) -> Result<FileEntry, VLFSError<F::Error>> {
    let address = at.address_of_file_entry(i);
    let mut buffer = [0u8; 5 + FILE_ENTRY_SIZE];
    let mut dummy_crc = DummyCrc {};
    let mut reader = FlashReader::new(address, flash, &mut dummy_crc);

    let (read_result, _) = reader
        .read_slice(&mut buffer, FILE_ENTRY_SIZE)
        .await
        .map_err(VLFSError::FlashError)?;

    Ok(FileEntry::deserialize(read_result)?)
}

pub struct FilesIterator<'a, F, P>
where
    F: Flash,
    P: FileEntryFilter,
{
    i: u16,
    at: RwLockReadGuard<'a, NoopRawMutex, AllocationTable, 10>,
    flash: RwLockReadGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,
    filter: P,
}

impl<'a, F, P> FilesIterator<'a, F, P>
where
    F: Flash,
    P: FileEntryFilter,
{
    async fn new(vlfs: &'a VLFS<F, impl Crc>, filter: P) -> Self {
        let at = vlfs.allocation_table.read().await;
        let flash = vlfs.flash.read().await;
        Self {
            i: 0,
            at,
            flash,
            filter,
        }
    }

    pub async fn next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        loop {
            if self.i >= self.at.footer.file_count {
                return Ok(None);
            }

            let file_entry = read_file_entry(self.i, &self.at, &mut self.flash).await;
            self.i += 1;
            let file_entry = file_entry?;

            if self.filter.check(&file_entry) {
                return Ok(Some(file_entry));
            }
        }
    }
}

pub struct ConcurrentFilesIterator<'a, F, C, P>
where
    F: Flash,
    C: Crc,
    P: FileEntryFilter,
{
    // file id and index of the last file entry
    last_file: Option<(FileID, u16)>,
    vlfs: &'a VLFS<F, C>,
    filter: P,
}

impl<'a, F, C, P> ConcurrentFilesIterator<'a, F, C, P>
where
    F: Flash,
    C: Crc,
    P: FileEntryFilter,
{
    async fn new(vlfs: &'a VLFS<F, C>, filter: P) -> Self {
        Self {
            last_file: None,
            vlfs,
            filter,
        }
    }

    // immediate next file entry, ignoring the predicate
    async fn immediate_next(
        &mut self,
        at: &RwLockReadGuard<'a, NoopRawMutex, AllocationTable, 10>,
        flash: &RwLockReadGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        if let Some((last_file_id, last_file_entry_i)) = self.last_file {
            let mut curr_plus_one_file_entry: Option<FileEntry> = None;

            // Try to read the next file entry first
            if last_file_entry_i + 1 < at.footer.file_count {
                let file_entry = read_file_entry(last_file_entry_i + 1, at, flash).await?;
                if file_entry.id.0 == last_file_id.0 + 1 {
                    // file_entry is the immediate next file entry of the last file
                    self.last_file = Some((file_entry.id, last_file_entry_i + 1));
                    return Ok(Some(file_entry));
                } else {
                    curr_plus_one_file_entry = Some(file_entry);
                }
            }
            // There are two other cases:
            // 1. file_entry.id.0 > last_file_id.0 + 1
            //     There could potentially be another file entry between the last file and the current file entry
            // 2. file_entry.id.0 < last_file_id.0 + 1
            //     This should never happen because when you create the file, the new file id is greater than all
            //     the past file ids

            // The following code handles case 1.
            if at.footer.file_count == 0 {
                self.last_file = None;
                return Ok(None);
            }
            let mut curr_file_entry_i = min(last_file_entry_i, at.footer.file_count - 1);
            // Find the first file with file id <= last file id, from curr_file_entry_i to 0
            // The file following the found file is the next file returned by the iterator
            loop {
                let curr_file_entry = read_file_entry(curr_file_entry_i, at, flash).await?;

                if curr_file_entry.id.0 <= last_file_id.0 {
                    if let Some(curr_plus_one_file_entry) = curr_plus_one_file_entry.take() {
                        self.last_file = Some((curr_plus_one_file_entry.id, curr_file_entry_i + 1));
                        return Ok(Some(curr_plus_one_file_entry));
                    } else {
                        // There are no files in the fs with id larger than last file id
                        return Ok(None);
                    }
                }

                if curr_file_entry_i == 0 {
                    // Reached the file with the smallest file id, but it is still larger than the last file id
                    self.last_file = Some((curr_file_entry.id, curr_file_entry_i));
                    return Ok(Some(curr_file_entry));
                }

                curr_plus_one_file_entry = Some(curr_file_entry);
                curr_file_entry_i -= 1;
            }
        } else {
            if at.footer.file_count > 0 {
                let file_entry = read_file_entry(0, at, flash).await?;
                self.last_file = Some((file_entry.id, 0));
                return Ok(Some(file_entry));
            } else {
                return Ok(None);
            }
        }
    }

    pub fn reset(&mut self) {
        self.last_file = None;
    }

    pub async fn next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        let at = self.vlfs.allocation_table.read().await;
        let flash = self.vlfs.flash.read().await;

        let immediate_next = self.immediate_next(&at, &flash).await?;
        if let Some(mut file_entry) = immediate_next {
            // last_file is guaranteed to be Some if immediate_next is Some
            let (last_file_id, last_file_entry_i) = self.last_file.as_mut().unwrap();
            loop {
                if self.filter.check(&file_entry) {
                    return Ok(Some(file_entry));
                }

                if *last_file_entry_i >= at.footer.file_count - 1 {
                    return Ok(None);
                }

                file_entry = read_file_entry(*last_file_entry_i + 1, &at, &flash).await?;
                *last_file_entry_i += 1;
                *last_file_id = file_entry.id;
            }
        } else {
            return Ok(None);
        }
    }
}
