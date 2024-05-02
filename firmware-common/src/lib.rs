#![cfg_attr(not(any(test, feature = "clap")), no_std)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(async_closure)]

mod fmt;

use common::utc_clock::UtcClockTask;
use driver::gps::{GPSParser, GPSPPS};
use embedded_hal_async::delay::DelayNs;
use futures::join;

use crate::{
    avionics::avionics_main,
    common::{
        console::{console::Console, programs::start_common_programs::start_common_programs},
        device_manager::prelude::*,
    },
    ground_test::gcm::ground_test_gcm,
};
use defmt::*;
use vlfs::{StatFlash, Timer as VLFSTimer, VLFS};

use futures::{future::select, pin_mut};

use crate::gcm::gcm_main;
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
mod gcm;
mod ground_test;
pub mod utils;

#[derive(Clone)]
struct VLFSTimerWrapper<T: Clock>(T);

impl<T: Clock> VLFSTimer for VLFSTimerWrapper<T> {
    fn now_ms(&self) -> f64 {
        self.0.now_ms()
    }
}

pub async fn testMain(
    clock: impl Clock,
    mut gps: impl GPS,
    pps: impl GPSPPS,
    mut delay: impl DelayNs,
) {
    gps.reset().await;

    let parser = GPSParser::new();
    let gps_parser_fut = parser.run(&mut gps);

    let utc_clock_task = UtcClockTask::new(clock);
    let utc_clock_task_fut = utc_clock_task.run(pps, &parser, clock);
    let utc_clock = utc_clock_task.get_clock();

    let display_fut = async {
        loop {
            delay.delay_ms(10).await;
            if utc_clock.ready() {
                info!("UTC: {}", (utc_clock.now_ms() / 1000.0) as i64);
            }else{
                // info!("UTC not ready");
            }
        }
    };

    join!(gps_parser_fut, utc_clock_task_fut, display_fut);
}

pub async fn init(
    device_manager: device_manager_type!(mut),
    device_mode_overwrite: Option<DeviceMode>,
) -> ! {
    claim_devices!(device_manager, flash, crc, usb, serial);
    let clock = device_manager.clock;
    let mut delay = device_manager.delay;

    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(clock));
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

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

    let serial_console = Console::<_, 20>::new(serial);
    let serial_console_common_programs_fut =
        start_common_programs(device_manager, &serial_console, &fs);
    let usb_console = Console::<_, 20>::new(usb);
    let usb_console_common_programs_fut = start_common_programs(device_manager, &usb_console, &fs);

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

        // info!("Starting in mode GROUND TEST");
        // ground_test_avionics(device_manager).await;
        // ground_test_gcm(device_manager).await;

        info!("Starting in mode {}", device_mode);
        match device_mode {
            DeviceMode::Avionics => avionics_main(&fs, device_manager).await,
            DeviceMode::GCM => {
                gcm_main::<20, 20>(&fs, device_manager, &serial_console, &usb_console).await
            }
            DeviceMode::BeaconSender => beacon_sender(&fs, device_manager, false).await,
            DeviceMode::BeaconReceiver => beacon_receiver(&fs, device_manager).await,
            DeviceMode::GroundTestAvionics => ground_test_avionics(device_manager).await,
            DeviceMode::GroundTestGCM => ground_test_gcm(device_manager).await,
        };
    };

    #[allow(unreachable_code)]
    {
        join!(
            main_fut,
            serial_console.run_dispatcher(),
            usb_console.run_dispatcher(),
            serial_console_common_programs_fut,
            usb_console_common_programs_fut
        );
    }

    defmt::unreachable!()
}
