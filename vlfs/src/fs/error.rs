use crate::driver::flash::Flash;

#[derive(defmt::Format)]
pub enum VLFSError<F: Flash> {
    FlashError(F::Error),
    FileAlreadyExists,
    MaxFilesReached,
    FileInUse,
    FileDoesNotExist,
    DeviceFull,
    WritingQueueFull,
    CorruptedPage { address: u32 },
    FileClosed,
}

impl<F: Flash> VLFSError<F> {
    pub fn from_flash(value: F::Error) -> Self {
        Self::FlashError(value)
    }
}
