#[derive(defmt::Format)]
pub enum VLFSError<FlashError: defmt::Format> {
    FlashError(FlashError),
    FileAlreadyExists,
    MaxFilesReached,
    FileInUse,
    FileDoesNotExist,
    DeviceFull,
    WritingQueueFull,
    CorruptedPage { address: u32 },
    FileClosed,
}
