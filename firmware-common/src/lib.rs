#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(let_chains)]
#![feature(generic_const_exprs)]

use defmt::*;
use driver::{crc::Crc, flash::SpiFlash};

use crate::common::vlfs::VLFS;

mod avionics;
mod common;
pub mod driver;
mod gcm;

pub async fn init<F: SpiFlash, C: Crc>(flash: F, crc: C) {
    let mut fs = VLFS::new(flash, crc);
    info!("Initializing VLFS");
    fs.init().await;
    info!("VLFS initialized");
}
