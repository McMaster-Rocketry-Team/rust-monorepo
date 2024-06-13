#![cfg_attr(not(any(test, feature = "clap")), no_std)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(async_closure)]
#![feature(assert_matches)]

mod fmt;

use common::{
    buzzer_queue::BuzzerQueueRunner, console::console::run_console, unix_clock::UnixClockTask,
};
use driver::gps::{GPSParser, GPSPPS};
use futures::join;

use crate::{
    avionics::avionics_main, common::device_manager::prelude::*, ground_test::gcm::ground_test_gcm,
};
use defmt::*;
use vlfs::{StatFlash, Timer as VLFSTimer, VLFS};

use futures::{future::select, pin_mut};

// use crate::gcm::gcm_main;
use crate::ground_test::avionics::ground_test_avionics;
use crate::{
    beacon::{beacon_receiver::beacon_receiver, beacon_sender::beacon_sender},
    common::device_mode::{read_device_mode, write_device_mode},
};
pub use common::device_manager::DeviceManager;
pub use common::device_mode::DeviceMode;

pub use common::vlp;
mod allocator;
mod avionics;
mod beacon;
mod common;
pub mod driver;
// mod gcm;
mod ground_test;
pub mod utils;

#[derive(Clone)]
struct VLFSTimerWrapper<T: Clock>(T);

impl<T: Clock> VLFSTimer for VLFSTimerWrapper<T> {
    fn now_ms(&self) -> f64 {
        self.0.now_ms()
    }
}

pub async fn init(
    device_manager: device_manager_type!(mut),
    device_mode_overwrite: Option<DeviceMode>,
) -> ! {
    claim_devices!(
        device_manager,
        flash,
        crc,
        usb,
        serial,
        buzzer,
        gps,
        gps_pps
    );
    let mut serial = serial.take().unwrap();
    let mut usb = usb.take().unwrap();
    let clock = device_manager.clock;
    let mut delay = device_manager.delay;

    // Start VLFS
    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(clock));
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    // Start GPS (provides unix time)
    gps.reset().await;
    let parser = GPSParser::new();
    let gps_parser_fut = parser.run(&mut gps);
    let unix_clock_task = UnixClockTask::new(clock);
    let unix_clock_task_fut = unix_clock_task.run(gps_pps, &parser, clock);
    let unix_clock = unix_clock_task.get_clock();

    // Buzzer Queue
    let buzzer_queue_runner = BuzzerQueueRunner::new();
    let buzzer_queue_runner_fut = buzzer_queue_runner.run(buzzer, delay);
    let buzzer_queue = buzzer_queue_runner.get_queue();

    let usb_connected = {
        let timeout_fut = delay.delay_ms(500);
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

    let serial_console_fut = run_console(&mut serial, &fs);
    let usb_console_fut = run_console(&mut usb, &fs);

    let main_fut = async {
        if usb_connected {
            info!("USB connected on boot, stopping main");
            claim_devices!(device_manager, status_indicator, error_indicator);
            loop {
                status_indicator.set_enable(true).await;
                error_indicator.set_enable(false).await;
                delay.delay_ms(1000).await;
                status_indicator.set_enable(false).await;
                error_indicator.set_enable(true).await;
                delay.delay_ms(1000).await;
            }
        }

        let device_mode = if let Some(device_mode_overwrite) = device_mode_overwrite {
            log_info!("Using device mode overwrite: {:?}", device_mode_overwrite);
            device_mode_overwrite
        } else {
            if let Some(device_mode_) = read_device_mode(&fs).await {
                log_info!("Read device mode from disk: {:?}", device_mode_);
                device_mode_
            } else {
                log_info!("No device mode file found, creating one");
                try_or_warn!(write_device_mode(&fs, DeviceMode::Avionics).await);
                DeviceMode::Avionics
            }
        };

        info!("Starting in mode {}", device_mode);
        match device_mode {
            DeviceMode::Avionics => avionics_main(&fs, device_manager).await,
            DeviceMode::GCM => {
                defmt::todo!();
                // gcm_main::<20, 20>(&fs, device_manager, &serial_console, &usb_console).await
            }
            DeviceMode::BeaconSender => beacon_sender(&fs, device_manager, false).await,
            DeviceMode::BeaconReceiver => beacon_receiver(&fs, device_manager).await,
            DeviceMode::GroundTestAvionics => {
                ground_test_avionics(&fs, unix_clock, &buzzer_queue, device_manager).await
            }
            DeviceMode::GroundTestGCM => ground_test_gcm(device_manager).await,
        };
    };

    #[allow(unreachable_code)]
    {
        join!(
            buzzer_queue_runner_fut,
            gps_parser_fut,
            unix_clock_task_fut,
            main_fut,
            serial_console_fut,
            usb_console_fut,
        );
    }

    defmt::unreachable!()
}
