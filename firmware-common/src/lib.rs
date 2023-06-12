#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use crate::{
    avionics::avionics_main,
    common::{
        console::console::run_console, device_manager::prelude::*, files::CALIBRATION_FILE_TYPE,
    },
    driver::debugger::DebuggerEvent,
    ground_test::gcm::ground_test_gcm,
};
use defmt::*;
use ferraris_calibration::CalibrationInfo;
use vlfs::{io_traits::AsyncReader, StatFlash, VLFS};

use futures::{
    future::{join4, select},
    pin_mut,
};

use crate::driver::timer::VLFSTimerWrapper;
use crate::gcm::gcm_main;
use crate::ground_test::avionics::ground_test_avionics;
use crate::{
    beacon::{beacon_receiver::beacon_receiver, beacon_sender::beacon_sender},
    common::device_mode::{read_device_mode, write_device_mode},
};
pub use common::device_manager::DeviceManager;
pub use common::device_mode::DeviceMode;
mod allocator;
mod avionics;
mod beacon;
mod common;
pub mod driver;
mod gcm;
mod ground_test;
pub mod utils;

pub async fn init(
    device_manager: device_manager_type!(mut),
    device_mode_overwrite: Option<DeviceMode>,
) -> ! {
    device_manager.init().await;
    claim_devices!(device_manager, flash, crc, usb, serial);
    let timer = device_manager.timer;

    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(timer));
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    let testing_fut = async {
        let mut iter = fs.files_iter(Some(CALIBRATION_FILE_TYPE)).await;
        let calibration_file = iter.next();
        drop(iter);
        if let Some(calibration_file) = calibration_file {
            info!("Calibration file found, id: {}", calibration_file.file_id);
            let mut file = unwrap!(fs.open_file_for_read(calibration_file.file_id).await);
            let mut buffer = [0u8; 156];
            unwrap!(file.read_slice(&mut buffer, 156).await);
            file.close().await;
            let cal_info = CalibrationInfo::deserialize(buffer);
            device_manager
                .debugger
                .clone()
                .dispatch(DebuggerEvent::CalInfo(cal_info.clone()));
            info!("{}", cal_info);
        } else {
            info!("No calibration file found");
        }
        // claim_devices!(device_manager, imu);
        // unwrap!(imu.wait_for_power_on().await);
        // unwrap!(imu.reset().await);
        // let mut ticker = Ticker::every(timer, 100.0);
        // unwrap!(meg.reset().await);
        // info!("meg resetted");
        // let start_time = timer.now_mills();
        // loop {
        //     info!("imu: {}", imu.read().await);
        //     ticker.next().await;
        // }
        // info!(
        //     "Time taken: {}",
        //     (timer.now_mills() - start_time) / 10.0
        // );
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

    let serial_console = run_console(&fs, serial, &stat_flash, device_manager);
    let usb_console = run_console(&fs, usb, &stat_flash, device_manager);

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

        let device_mode = if let Some(device_mode_overwrite) = device_mode_overwrite {
            info!("Using device mode overwrite: {:?}", device_mode_overwrite);
            device_mode_overwrite
        } else {
            if let Some(device_mode_) = read_device_mode(&fs).await {
                info!("Read device mode from disk: {}", device_mode_);
                device_mode_
            } else {
                info!("No device mode file found, creating one");
                try_or_warn!(write_device_mode(&fs, DeviceMode::Avionics).await);
                DeviceMode::Avionics
            }
        };

        // info!("Starting in mode GROUND TEST");
        // ground_test_avionics(device_manager).await;
        // ground_test_gcm(device_manager).await;

        info!("Starting in mode {}", device_mode);
        match device_mode {
            DeviceMode::Avionics => avionics_main(&fs, device_manager).await,
            DeviceMode::GCM => gcm_main(&fs, device_manager).await,
            DeviceMode::BeaconSender => beacon_sender(&fs, device_manager, false).await,
            DeviceMode::BeaconReceiver => beacon_receiver(&fs, device_manager).await,
            DeviceMode::GroundTestAvionics => ground_test_avionics(device_manager).await,
            DeviceMode::GroundTestGCM => ground_test_gcm(device_manager).await,
        };
    };

    join4(main_fut, serial_console, usb_console, testing_fut).await;

    defmt::unreachable!()
}
