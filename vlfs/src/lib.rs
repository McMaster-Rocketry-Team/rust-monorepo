#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(exclusive_range_pattern)]

mod fmt;

pub use driver::async_erase_flash::AsyncEraseFlash;
pub use driver::crc::Crc;
pub use driver::dummy_crc::DummyCrc;
pub use driver::dummy_flash::DummyFlash;
pub use driver::flash::Flash;
pub use driver::managed_erase_flash::{EraseTune, ManagedEraseFlash};
pub use driver::stat_flash::{Stat, StatFlash, StatFlashFlash};
pub use driver::timer::Timer;
pub use fs::allocation_table::FileEntry;
pub use fs::error::VLFSError;
pub use fs::iter::FilesIterator;
pub use fs::reader::{FileReader, VLFSReadStatus};
pub use fs::writer::FileWriter;
pub use fs::{FileID, FileType, VLFS};
pub use utils::io_traits;
pub use fs::init;

mod driver;
mod fs;
mod utils;
