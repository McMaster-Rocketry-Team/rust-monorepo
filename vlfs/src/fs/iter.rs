use async_iterator::Iterator;

use crate::{io_traits::AsyncReader, utils::flash_io::FlashReader, DummyCrc};

use super::*;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    /// Deleting files while iterating over the files is supported.
    /// Creating files while iterating over the files is not supported. (will skip a file)
    pub async fn files_iter(&self) -> FilesIterator<F, C> {
        let at = self.allocation_table.read().await;
        FilesIterator {
            reverse_i: at.file_count,
            vlfs: self,
        }
    }

    pub async fn find_file_by_type(&self, file_type: FileType) -> Option<FileEntry> {
        let mut iter = self.files_iter().await;
        while let Some(file_entry) = iter.next().await {
            if let Ok(file_entry) = file_entry {
                if file_entry.typ == file_type {
                    return Some(file_entry);
                }
            } else {
                log_warn!("skipping corropted file entry");
            }
        }
        None
    }
}

pub struct FilesIterator<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    // using reverse_i so that we can remove files while iterating
    reverse_i: u16,
    vlfs: &'a VLFS<F, C>,
}

impl<'a, F, C> async_iterator::Iterator for FilesIterator<'a, F, C>
where
    F: Flash,
    C: Crc,
{
    type Item = Result<FileEntry, VLFSError<F::Error>>;

    async fn next(&mut self) -> Option<Self::Item> {
        if self.reverse_i == 0xFFFF {
            return None;
        }
        let at = self.vlfs.allocation_table.read().await;
        if self.reverse_i > at.file_count {
            return None;
        }
        let i = at.file_count - self.reverse_i;

        let address = at.address_of_file_entry(i);
        let mut flash = self.vlfs.flash.lock().await;
        let mut buffer = [0u8; 5 + 13];
        let mut dummy_crc = DummyCrc {};
        let mut reader = FlashReader::new(address, &mut flash, &mut dummy_crc);

        match reader
            .read_slice(&mut buffer, 13)
            .await
            .map_err(VLFSError::FlashError)
        {
            Ok((read_result, _)) => {
                if self.reverse_i == 0 {
                    self.reverse_i = 0xFFFF;
                } else {
                    self.reverse_i -= 1;
                }
                match FileEntry::deserialize(read_result) {
                    Ok(mut file_entry) => {
                        file_entry.opened = self.vlfs.is_file_opened(file_entry.id).await;
                        return Some(Ok(file_entry));
                    }
                    Err(e) => return Some(Err(e.into())),
                }
            }
            Err(e) => return Some(Err(e)),
        };
    }
}
