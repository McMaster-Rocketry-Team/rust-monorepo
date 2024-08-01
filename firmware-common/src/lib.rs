// only use std when feature = "std" is enabled or during testing
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![feature(const_trait_impl)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(async_closure)]
#![feature(assert_matches)]
#![feature(never_type)]

mod fmt;

use common::config_file::ConfigFile;
use common::console::rpc::run_rpc_server;
use common::device_config::{DeviceConfig, DeviceModeConfig};
use common::device_manager::SystemServices;
use common::file_types::DEVICE_CONFIG_FILE_TYPE;
use common::rpc_channel::RpcChannel;
use common::sensor_reading::SensorReading;
use common::{buzzer_queue::BuzzerQueueRunner, unix_clock::UnixClockTask};
use driver::clock::VLFSTimerWrapper;
use driver::gps::GPSData;
use driver::timestamp::BootTimestamp;
use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use futures::join;

use crate::{
    avionics::avionics_main, common::device_manager::prelude::*, ground_test::gcm::ground_test_gcm,
};
use vlfs::{StatFlash, VLFS};

use futures::{future::select, pin_mut};

use crate::gcm::gcm_main;
use crate::ground_test::avionics::ground_test_avionics;
pub use common::device_manager::DeviceManager;

pub use common::console::rpc::RpcClient;
mod avionics;
pub mod common;
pub mod driver;
mod gcm;
mod ground_test;
pub mod utils;

pub async fn init(
    device_manager: device_manager_type!(mut),
    device_serial_number: &[u8; 12],
    device_config: Option<DeviceConfig>,
) -> ! {
    #[cfg(all(feature = "defmt", feature = "log"))]
    compile_error!("Feature defmt and log are mutually exclusive and cannot be enabled together");

    #[cfg(all(feature = "std", feature = "global-allocator"))]
    compile_error!(
        "Feature std and global-allocator are mutually exclusive and cannot be enabled together"
    );

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

    // Start VLFS
    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(device_manager.clock()));
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    fs.init().await.unwrap();

    // Start GPS (provides unix time)
    let gps_timestamp_pubsub = PubSubChannel::<NoopRawMutex, i64, 1, 1, 1>::new();
    let gps_location_pubsub =
        PubSubChannel::<NoopRawMutex, SensorReading<BootTimestamp, GPSData>, 1, 1, 1>::new();
    let gps_distribution_fut = async {
        loop {
            match gps.next_location().await {
                Ok(gps_reading) => {
                    if let Some(gps_timestamp) = gps_reading.data.timestamp {
                        gps_timestamp_pubsub.publish_immediate(gps_timestamp);
                    }
                    gps_location_pubsub.publish_immediate(gps_reading);
                }
                Err(e) => {
                    log_error!("Error reading GPS: {:?}", e);
                    yield_now().await;
                }
            }
        }
    };
    let unix_clock_task = UnixClockTask::new(device_manager.clock());
    let unix_clock_task_fut = unix_clock_task.run(
        gps_pps,
        gps_timestamp_pubsub.subscriber().unwrap(),
        device_manager.clock(),
    );
    let unix_clock = unix_clock_task.get_clock();

    // Buzzer Queue
    let buzzer_queue_runner = BuzzerQueueRunner::new();
    let buzzer_queue_runner_fut = buzzer_queue_runner.run(buzzer, device_manager.delay());
    let buzzer_queue = buzzer_queue_runner.get_queue();

    let services = SystemServices {
        fs: &fs,
        gps: &gps_location_pubsub,
        delay: device_manager.delay(),
        clock: device_manager.clock(),
        unix_clock,
        buzzer_queue,
    };

    let delay = device_manager.delay();
    let usb_connected = {
        let timeout_fut = delay.delay_ms(500.0);
        let usb_wait_connection_fut = usb.wait_connection();
        pin_mut!(timeout_fut);
        pin_mut!(usb_wait_connection_fut);
        match select(timeout_fut, usb_wait_connection_fut).await {
            futures::future::Either::Left(_) => {
                log_info!("USB not connected");
                false
            }
            futures::future::Either::Right(_) => {
                log_info!("USB connected");
                true
            }
        }
    };

    let device_config = if let Some(device_config) = device_config {
        log_info!("Using device config overwrite");
        Some(device_config)
    } else {
        let device_config_file = ConfigFile::new(services.fs, DEVICE_CONFIG_FILE_TYPE);
        if let Some(device_config) = device_config_file.read().await {
            log_info!("Read device mode from disk");
            Some(device_config)
        } else {
            log_info!("No device mode file found");
            None
        }
    };

    let gcm_downlink_package_channel = Channel::new();
    let gcm_send_uplink_packet_rpc = RpcChannel::new();

    let serial_console_fut = run_rpc_server(
        &mut serial,
        &services,
        &device_config,
        device_serial_number,
        gcm_downlink_package_channel.receiver(),
        gcm_send_uplink_packet_rpc.client(),
    );
    let usb_console_fut = async {
        loop {
            usb.wait_connection().await;
            run_rpc_server(
                &mut usb,
                &services,
                &device_config,
                device_serial_number,
                gcm_downlink_package_channel.receiver(),
                gcm_send_uplink_packet_rpc.client(),
            )
            .await;
        }
    };

    let main_fut = async {
        // if usb_connected {
        //     log_info!("USB connected on boot, stopping main");
        //     claim_devices!(device_manager, indicators);
        //     indicators.run([1000, 1000], [0, 1000, 1000, 0], []).await;
        // }

        if device_config.as_ref().is_none() {
            log_info!("No device mode file found, halting");
            claim_devices!(device_manager, indicators);
            indicators
                .run([333, 666], [0, 333, 333, 333], [0, 666, 333, 0])
                .await;
        }
        let device_config = device_config.as_ref().unwrap();

        log_info!("Starting in mode {:?}", device_config);
        match device_config.mode {
            DeviceModeConfig::Avionics { .. } => {
                avionics_main(
                    device_manager,
                    &services,
                    &device_config,
                    device_serial_number,
                )
                .await
            }
            DeviceModeConfig::GCM { .. } => {
                gcm_main(
                    device_manager,
                    &services,
                    &device_config,
                    gcm_downlink_package_channel.sender(),
                    gcm_send_uplink_packet_rpc.server(),
                )
                .await
            }
            DeviceModeConfig::GroundTestAvionics{..} => {
                ground_test_avionics(device_manager, &services, &device_config).await
            }
            DeviceModeConfig::GroundTestGCM => ground_test_gcm(device_manager).await,
        };
    };

    #[allow(unreachable_code)]
    {
        join!(
            buzzer_queue_runner_fut,
            gps_distribution_fut,
            unix_clock_task_fut,
            main_fut,
            serial_console_fut,
            usb_console_fut,
        );
    }

    log_unreachable!()
}
