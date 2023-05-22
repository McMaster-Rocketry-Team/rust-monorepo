#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use crate::common::{console::console::run_console, device_manager::prelude::*, ticker::Ticker};
use defmt::*;
use vlfs::VLFS;

use futures::{
    future::{join4, select},
    pin_mut,
};

use crate::{
    beacon::{beacon_receiver::beacon_receiver, beacon_sender::beacon_sender},
    common::device_mode::{read_device_mode, write_device_mode, DeviceMode},
};

pub use common::device_manager::DeviceManager;

mod allocator;
mod avionics;
mod beacon;
mod common;
pub mod driver;
mod gcm;
mod utils;

pub async fn init(device_manager: device_manager_type!(mut)) -> ! {
    device_manager.init().await;
    claim_devices!(device_manager, flash, crc, usb, serial);
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    let timer = device_manager.timer;

    let testing_fut = async {
        claim_devices!(device_manager, meg);
        // unwrap!(imu.wait_for_power_on().await);
        // unwrap!(imu.reset().await);
        // let mut ticker = Ticker::every(timer, 1.0);
        // unwrap!(meg.reset().await);
        info!("meg resetted");
        let start_time = timer.now_mills();
        loop {
            info!("meg: {}",meg.reset().await);
        }
        info!(
            "Time taken: {}",
            (timer.now_mills() - start_time) / 10.0
        );
    };

    let usb_connected = {
        let timeout_fut = timer.sleep(500.0);
        let usb_wait_connection_fut = usb.wait_connection();
        pin_mut!(timeout_fut);
        pin_mut!(usb_wait_connection_fut);
        match select(timeout_fut, usb_wait_connection_fut).await {
            futures::future::Either::Left(_) => {
                info!("USB not connected");
                false
            }
            futures::future::Either::Right(_) => {
                info!("USB connected");
                true
            }
        }
    };

    let serial_console = run_console(&fs, serial, device_manager);
    let usb_console = run_console(&fs, usb, device_manager);

    let main_fut = async {
        if usb_connected {
            info!("USB connected on boot, stopping main");
            claim_devices!(device_manager, status_indicator, error_indicator);
            loop {
                status_indicator.set_enable(true).await;
                error_indicator.set_enable(false).await;
                timer.sleep(1000.0).await;
                status_indicator.set_enable(false).await;
                error_indicator.set_enable(true).await;
                timer.sleep(1000.0).await;
            }
        }

        let mut device_mode = DeviceMode::BeaconSender;
        if let Some(device_mode_) = read_device_mode(&fs).await {
            info!("Read device mode from disk: {}", device_mode_);
            device_mode = device_mode_;
        } else {
            info!("No device mode file found, creating one");
            try_or_warn!(write_device_mode(&fs, device_mode).await);
        }

        info!("Starting in mode {}", device_mode);
        match device_mode {
            DeviceMode::Avionics => defmt::panic!("Avionics mode not implemented"),
            DeviceMode::GCM => defmt::panic!("GCM mode not implemented"),
            DeviceMode::BeaconSender => beacon_sender(&fs, device_manager, false).await,
            DeviceMode::BeaconReceiver => beacon_receiver(&fs, device_manager).await,
        };
    };

    join4(main_fut, serial_console, usb_console, testing_fut).await;

    defmt::unreachable!()
}
