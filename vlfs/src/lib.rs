#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

pub use driver::crc::Crc;
pub use driver::dummy_crc::DummyCrc;
pub use driver::dummy_flash::DummyFlash;
pub use driver::flash::Flash;
pub use fs::error::VLFSError;
pub use fs::iter::{FilesIterator, LsFileEntry};
pub use fs::reader::{FileReader, VLFSReadStatus};
pub use fs::writer::FileWriter;
pub use fs::{FileID, FileType, VLFS};
pub use utils::io_traits;

mod driver;
mod fs;
mod utils;
