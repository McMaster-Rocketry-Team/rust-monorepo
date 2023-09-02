use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    /// Trying to create or delete files while iterating over the files will result in a deadlock.
    ///
    /// To delete multiple files, use [`remove_files`](Self::remove_files) instead.
    pub fn files_iter(&self, file_type: Option<FileType>) -> FilesIterator<F, C> {
        FilesIterator {
            i: 0,
            vlfs: self,
            file_type,
        }
    }

    pub async fn find_file_by_type(&self, file_type: FileType) -> Option<FileEntry> {
        let mut iter = self.files_iter(Some(file_type));
        let file = iter.next();
        drop(iter);
        file
    }
}

pub struct FilesIterator<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    i: usize,
    vlfs: &'a VLFS<F, C>,
    file_type: Option<FileType>,
}

impl<'a, F, C> FilesIterator<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn len(&self) -> usize {
        self.vlfs.allocation_table.read().await.file_count as usize
    }
}

impl<'a, F, C> Iterator for FilesIterator<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Item = FileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        defmt::todo!()
        // let file_entries = &self.at.allocation_table.file_entries;

        // if let Some(file_type) = self.file_type {
        //     while self.i < file_entries.len() {
        //         let entry = &file_entries[self.i];
        //         self.i += 1;

        //         if entry.file_type == file_type {
        //             return Some(entry.into());
        //         }
        //     }

        //     return None;
        // } else {
        //     let result = file_entries.get(self.i).map(|entry| entry.into());
        //     self.i += 1;
        //     return result;
        // }
    }
}
