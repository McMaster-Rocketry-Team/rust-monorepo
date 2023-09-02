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
    FileClosed,
}
