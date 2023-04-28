use crate::driver::flash::SpiFlashError;

#[derive(Debug, defmt::Format)]
pub enum VLFSError {
    FlashError(SpiFlashError),
    FileAlreadyExists,
    MaxFilesReached,
    FileInUse,
    FileDoesNotExist,
    DeviceFull,
    WritingQueueFull,
    CorruptedPage { address: u32 },
}

impl From<SpiFlashError> for VLFSError {
    fn from(value: SpiFlashError) -> Self {
        Self::FlashError(value)
    }
}
