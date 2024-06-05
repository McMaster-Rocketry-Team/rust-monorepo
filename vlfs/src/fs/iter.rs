use core::cmp::min;

use allocation_table::FILE_ENTRY_SIZE;
use embassy_sync::mutex::MutexGuard;

use crate::{
    utils::{flash_io::FlashReader, rwlock::RwLockReadGuard},
    AsyncReader, DummyCrc,
};

use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    /// Creating or deleting files while iterating over the files is NOT supported.
    /// The files returned by this iterator are guaranteed to be in increasing order of file id.
    /// If you need to create or delete files while iterating, use `concurrent_files_iter`.
    pub async fn files_iter(&self) -> FilesIterator<F, fn(&FileEntry) -> bool> {
        FilesIterator::new(&self, None).await
    }

    pub async fn files_iter_filter<P: FnMut(&FileEntry) -> bool>(
        &self,
        predicate: P,
    ) -> FilesIterator<F, P> {
        FilesIterator::new(&self, Some(predicate)).await
    }

    /// Creating or deleting files while iterating over the files is supported.
    /// The files returned by this iterator are guaranteed to be in increasing order of file id.
    /// This is less efficient than `files_iter`.
    pub async fn concurrent_files_iter(
        &self,
    ) -> ConcurrentFilesIterator<F, C, fn(&FileEntry) -> bool> {
        ConcurrentFilesIterator::new(&self, None).await
    }

    pub async fn concurrent_files_iter_filter<P: FnMut(&FileEntry) -> bool>(
        &self,
        predicate: P,
    ) -> ConcurrentFilesIterator<F, C, P> {
        ConcurrentFilesIterator::new(&self, Some(predicate)).await
    }

    /// Find the first file (the file that has the smallest file id) that satisfies the predicate.
    pub async fn find_first_file(
        &self,
        predicate: impl Fn(&FileEntry) -> bool,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        let mut iter = self.files_iter_filter(predicate).await;
        iter.next().await
    }

    pub async fn find_first_file_by_type(
        &self,
        file_type: FileType,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        self.find_first_file(|file_entry| file_entry.typ == file_type)
            .await
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
    P: FnMut(&FileEntry) -> bool,
{
    i: u16,
    at: RwLockReadGuard<'a, NoopRawMutex, AllocationTable, 10>,
    flash: RwLockReadGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,
    predicate: Option<P>,
}

impl<'a, F, P> FilesIterator<'a, F, P>
where
    F: Flash,
    P: FnMut(&FileEntry) -> bool,
{
    async fn new(vlfs: &'a VLFS<F, impl Crc>, predicate: Option<P>) -> Self {
        let at = vlfs.allocation_table.read().await;
        let flash = vlfs.flash.read().await;
        Self {
            i: 0,
            at,
            flash,
            predicate,
        }
    }

    pub async fn next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        loop {
            if self.i >= self.at.header.file_count {
                return Ok(None);
            }

            let file_entry = read_file_entry(self.i, &self.at, &mut self.flash).await;
            self.i += 1;
            let file_entry = file_entry?;

            if let Some(predicate) = &mut self.predicate {
                if predicate(&file_entry) {
                    return Ok(Some(file_entry));
                }
            } else {
                return Ok(Some(file_entry));
            }
        }
    }
}

pub struct ConcurrentFilesIterator<'a, F, C, P>
where
    F: Flash,
    C: Crc,
    P: FnMut(&FileEntry) -> bool,
{
    // file id and index of the last file entry
    last_file: Option<(FileID, u16)>,
    vlfs: &'a VLFS<F, C>,
    predicate: Option<P>,
}

impl<'a, F, C, P> ConcurrentFilesIterator<'a, F, C, P>
where
    F: Flash,
    C: Crc,
    P: FnMut(&FileEntry) -> bool,
{
    async fn new(vlfs: &'a VLFS<F, C>, predicate: Option<P>) -> Self {
        Self {
            last_file: None,
            vlfs,
            predicate,
        }
    }

    // immediate next file entry, ignoring the predicate
    async fn immediate_next(
        &mut self,
        at: &RwLockReadGuard<'a, NoopRawMutex, AllocationTable, 10>,
        flash: &RwLockReadGuard<'a, NoopRawMutex, FlashWrapper<F>, 10>,
    ) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        if let Some((last_file_id, last_file_entry_i)) = self.last_file {
            // Try to read the next file entry first
            if last_file_entry_i + 1 < at.header.file_count {
                let file_entry = read_file_entry(last_file_entry_i + 1, at, flash).await?;
                if file_entry.id.0 == last_file_id.0 + 1 {
                    // file_entry is the immediate next file entry of the last file
                    self.last_file = Some((file_entry.id, last_file_entry_i + 1));
                    return Ok(Some(file_entry));
                }
            }
            // There are two other cases:
            // 1. file_entry.id.0 > last_file_id.0 + 1
            //     There could potentially be another file entry between the last file and the current file entry
            // 2. file_entry.id.0 < last_file_id.0 + 1
            //     This should never happen because when you create the file, the new file id is greater than all
            //     the past file ids

            // The following code handles case 1.
            if at.header.file_count == 0 {
                self.last_file = None;
                return Ok(None);
            }
            let mut curr_file_entry_i = min(last_file_entry_i, at.header.file_count - 1);
            let mut curr_plus_one_file_entry: Option<FileEntry> = None;
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
            if at.header.file_count > 0 {
                let file_entry = read_file_entry(0, at, flash).await?;
                self.last_file = Some((file_entry.id, 0));
                return Ok(Some(file_entry));
            } else {
                return Ok(None);
            }
        }
    }

    pub async fn next(&mut self) -> Result<Option<FileEntry>, VLFSError<F::Error>> {
        let at = self.vlfs.allocation_table.read().await;
        let flash = self.vlfs.flash.read().await;

        let immediate_next = self.immediate_next(&at, &flash).await?;
        if let Some(mut file_entry) = immediate_next {
            if let Some(predicate) = &mut self.predicate {
                // last_file is guaranteed to be Some is immediate_next is Some
                let (last_file_id, last_file_entry_i) = self.last_file.as_mut().unwrap();
                loop {
                    if predicate(&file_entry) {
                        return Ok(Some(file_entry));
                    }

                    if *last_file_entry_i >= at.header.file_count - 1 {
                        return Ok(None);
                    }

                    file_entry = read_file_entry(*last_file_entry_i + 1, &at, &flash).await?;
                    *last_file_entry_i += 1;
                    *last_file_id = file_entry.id;
                }
            } else {
                return Ok(Some(file_entry));
            }
        } else {
            return Ok(None);
        }
    }
}
