#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

use crate::common::{console::console::run_console, device_manager::prelude::*};
use defmt::*;
use vlfs::VLFS;

use futures::{
    future::{join3, select},
    pin_mut,
};

use crate::{
    beacon::{beacon_receiver::beacon_receiver, beacon_sender::beacon_sender},
    common::{
        device_mode::{read_device_mode, write_device_mode, DeviceMode},
        gps_parser::GPSParser,
    },
};

pub use common::device_manager::DeviceManager;

mod allocator;
mod avionics;
mod beacon;
mod common;
pub mod driver;
mod gcm;
mod utils;

pub async fn init(device_manager: device_manager_type!()) -> ! {
    claim_devices!(device_manager, flash, crc);
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    let timer = device_manager.timer();
    claim_devices!(device_manager, usb);

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

    claim_devices!(device_manager, serial);
    let serial_console = run_console(&fs, serial, device_manager);
    let usb_console = run_console(&fs, usb, device_manager);

    let main_fut = async {
        claim_devices!(
            device_manager,
            status_indicator,
            error_indicator,
            radio_kind,
            gps
        );
        // let calibrate = Calibrate::new();
        // try_or_warn!(
        //     calibrate
        //         .start(&mut serial, &mut imu, &mut buzzer, timer, &fs)
        //         .await
        // );

        if usb_connected {
            info!("USB connected on boot, stopping main");
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
            DeviceMode::BeaconSender => {
                beacon_sender(
                    timer,
                    &fs,
                    GPSParser::new(gps),
                    radio_kind,
                    status_indicator,
                    error_indicator,
                )
                .await
            }
            DeviceMode::BeaconReceiver => beacon_receiver(timer, &fs, radio_kind).await,
        };

        return_devices!(
            device_manager,
            status_indicator,
            error_indicator,
            radio_kind,
            gps
        );
    };

    join3(main_fut, serial_console, usb_console).await;

    defmt::panic!("wtf");
}
