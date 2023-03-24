#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(let_chains)]

use defmt::*;
use driver::flash::SpiFlash;
use avionics::avionics_storage::AvionicsStorage;

pub mod driver;
mod storage;
mod avionics;

pub async fn init<F: SpiFlash>(flash: F) {
    AvionicsStorage::new(flash).await;
    info!("Avionics storage initialized");
}
