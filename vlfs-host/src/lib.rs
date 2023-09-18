#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

pub use file_flash::FileFlash;

mod file_flash;
#[cfg(test)]
mod tests;
