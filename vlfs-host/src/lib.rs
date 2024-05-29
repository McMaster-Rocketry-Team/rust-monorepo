#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(is_sorted)]
#![feature(assert_matches)]

pub use file_flash::FileFlash;

mod file_flash;
mod memory_flash;
mod debug_flash;
#[cfg(test)]
mod tests;
