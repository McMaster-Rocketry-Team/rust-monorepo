// only use std when feature = "std" is enabled or during testing
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(assert_matches)]
#![feature(is_sorted)]
#![feature(async_closure)]
#![feature(exclusive_range_pattern)]

mod fmt;

pub use driver::timer::Timer;
pub use driver::crc::Crc;
pub use driver::dummy_crc::DummyCrc;
pub use driver::flash::Flash;

pub use flash::dummy_flash::DummyFlash;
pub use flash::async_erase_flash::AsyncEraseFlash;
pub use flash::managed_erase_flash::{EraseTune, ManagedEraseFlash};
pub use flash::stat_flash::{Stat, StatFlash, StatFlashFlash};
#[cfg(feature = "std")]
pub use flash::memory_flash::MemoryFlash;
#[cfg(feature = "std")]
pub use flash::file_flash::FileFlash;

pub use fs::allocation_table::FileEntry;
pub use fs::error::VLFSError;
pub use fs::iter::{FilesIterator, ConcurrentFilesIterator, FileEntryFilter};
pub use fs::reader::{FileReader, VLFSReadStatus};
pub use fs::writer::FileWriter;
pub use fs::{FileID, FileType, VLFS};
pub use utils::io_traits::{AsyncReader, AsyncWriter};

mod driver;
mod flash;
mod fs;
mod utils;
#[cfg(test)]
mod tests;