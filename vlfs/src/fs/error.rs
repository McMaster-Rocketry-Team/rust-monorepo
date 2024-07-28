use core::fmt::Debug;

use embedded_io_async::ErrorKind;

use super::allocation_table::CorruptedFileEntry;

#[derive(defmt::Format, Debug)]
pub enum VLFSError<FlashError: defmt::Format + Debug + embedded_io_async::Error> {
    FlashError(FlashError),
    FileAlreadyExists,
    TooManyFiles,
    TooManyFilesOpen,
    FileInUse,
    FileDoesNotExist,
    DeviceFull,
    CorruptedPage { address: u32 },
    CorruptedFileEntry,
    CorruptedFileSystem,
}

impl<FlashError: defmt::Format + Debug + embedded_io_async::Error> From<CorruptedFileEntry> for VLFSError<FlashError> {
    fn from(_: CorruptedFileEntry) -> Self {
        Self::CorruptedFileEntry
    }
}

impl<FlashError: defmt::Format + Debug + embedded_io_async::Error> embedded_io_async::Error for VLFSError<FlashError> {
    fn kind(&self) -> ErrorKind {
        match self {
            VLFSError::FlashError(e) => e.kind(),
            VLFSError::FileAlreadyExists => ErrorKind::AlreadyExists,
            VLFSError::TooManyFiles => ErrorKind::OutOfMemory,
            VLFSError::TooManyFilesOpen => ErrorKind::OutOfMemory,
            VLFSError::FileInUse => ErrorKind::AddrInUse,
            VLFSError::FileDoesNotExist => ErrorKind::NotFound,
            VLFSError::DeviceFull => ErrorKind::OutOfMemory,
            VLFSError::CorruptedPage { .. } => ErrorKind::Other,
            VLFSError::CorruptedFileEntry => ErrorKind::Other,
            VLFSError::CorruptedFileSystem => ErrorKind::Other,
        }
    }
}