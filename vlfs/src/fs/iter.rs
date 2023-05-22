use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    /// Trying to create or delete files while iterating over the files will result in a deadlock.
    ///
    /// To avoid the deadlock, see the following example:
    ///
    /// ```rust
    /// while let Some(file_entry) = fs.files_iter(Some(FileType(0))).await.next() {
    ///     fs.remove_file(file_entry.file_id).await?;
    /// }
    /// ```
    pub async fn files_iter(&self, file_type: Option<FileType>) -> FilesIterator {
        FilesIterator {
            i: 0,
            at: self.allocation_table.read().await,
            file_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LsFileEntry {
    pub file_id: FileID,
    pub file_type: FileType,
}

impl From<&FileEntry> for LsFileEntry {
    fn from(file_entry: &FileEntry) -> Self {
        Self {
            file_id: file_entry.file_id,
            file_type: file_entry.file_type,
        }
    }
}

pub struct FilesIterator<'a> {
    i: usize,
    at: RwLockReadGuard<'a, CriticalSectionRawMutex, AllocationTableWrapper, 10>,
    file_type: Option<FileType>,
}

impl<'a> FilesIterator<'a> {
    pub fn len(&self) -> usize {
        self.at.allocation_table.file_entries.len()
    }
}

impl<'a> Iterator for FilesIterator<'a> {
    type Item = LsFileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let file_entries = &self.at.allocation_table.file_entries;

        if let Some(file_type) = self.file_type {
            while self.i < file_entries.len() {
                let entry = &file_entries[self.i];
                self.i += 1;

                if entry.file_type == file_type {
                    return Some(entry.into());
                }
            }

            return None;
        } else {
            let result = file_entries.get(self.i).map(|entry| entry.into());
            self.i += 1;
            return result;
        }
    }
}
