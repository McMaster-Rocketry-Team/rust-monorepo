use super::allocation_table::CorruptedFileEntry;

#[derive(defmt::Format, Debug)]
pub enum VLFSError<FlashError: defmt::Format> {
    FlashError(FlashError),
    FileAlreadyExists,
    TooManyFiles,
    TooManyFilesOpen,
    FileInUse,
    FileDoesNotExist,
    DeviceFull,
    WritingQueueFull,
    CorruptedPage { address: u32 },
    CorruptedFileEntry,
    FileClosed,
}

impl<FlashError: defmt::Format> From<CorruptedFileEntry> for VLFSError<FlashError> {
    fn from(_: CorruptedFileEntry) -> Self {
        Self::CorruptedFileEntry
    }
}