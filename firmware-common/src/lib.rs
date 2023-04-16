#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]

use defmt::info;
use driver::{crc::Crc, flash::SpiFlash, imu::IMU, timer::Timer};

use crate::common::vlfs::VLFS;

mod avionics;
mod common;
pub mod driver;
mod gcm;

pub async fn init<T: Timer, F: SpiFlash, C: Crc, I: IMU>(timer: T, flash: F, crc: C, mut imu: I) {
    let mut fs = VLFS::new(flash, crc);
    fs.init().await;
    // loop {
    //     let reading = imu.read().await;
    //     info!("imu: {}", reading);
    //     timer.sleep(10).await;
    // }
}
