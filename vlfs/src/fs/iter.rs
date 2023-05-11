use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn files_iter(&self) -> FilesIterator {
        FilesIterator {
            i: 0,
            at: self.allocation_table.read().await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LsFileEntry {
    pub file_id: u64,
    pub file_type: u16,
}

pub struct FilesIterator<'a> {
    i: usize,
    at: RwLockReadGuard<'a, CriticalSectionRawMutex, AllocationTableWrapper, 10>,
}

impl<'a> FilesIterator<'a> {
    pub fn len(&self) -> usize {
        self.at.allocation_table.file_entries.len()
    }
}

impl<'a> Iterator for FilesIterator<'a> {
    type Item = LsFileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .at
            .allocation_table
            .file_entries
            .get(self.i)
            .map(|file_entry| LsFileEntry {
                file_id: file_entry.file_id,
                file_type: file_entry.file_type,
            });
        self.i += 1;
        result
    }
}
