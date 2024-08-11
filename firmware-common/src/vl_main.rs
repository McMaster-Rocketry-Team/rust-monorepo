use crate::claim_devices;
use crate::common::config_file::ConfigFile;
use crate::common::console::vl_rpc::run_rpc_server;
use crate::common::device_config::{DeviceConfig, DeviceModeConfig};
use crate::common::file_types::DEVICE_CONFIG_FILE_TYPE;
use crate::common::rpc_channel::RpcChannel;
use crate::common::sensor_reading::SensorReading;
use crate::common::{buzzer_queue::BuzzerQueueRunner, unix_clock::UnixClockTask};
use crate::driver::clock::VLFSTimerWrapper;
use crate::driver::gps::GPSData;
use crate::driver::timestamp::BootTimestamp;
use embassy_futures::select::{select, Either};
use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use futures::join;
use crate::vacuum_test::vacuum_test_main;

use crate::common::vl_device_manager::prelude::*;
use vlfs::{StatFlash, VLFS};

use crate::gcm::gcm_main;
use crate::ground_test_avionics::ground_test_avionics;

pub async fn vl_main(
    device_manager: vl_device_manager_type!(mut),
    device_serial_number: &[u8; 12],
    device_config: Option<DeviceConfig>,
) -> ! {
    #[cfg(all(feature = "defmt", feature = "log"))]
    compile_error!("Feature defmt and log are mutually exclusive and cannot be enabled together");

    claim_devices!(
        device_manager,
        flash,
        crc,
        usb,
        serial,
        buzzer,
        gps,
        gps_pps,
        sys_reset
    );
    let mut serial = serial.take().unwrap();
    let mut usb = usb.take().unwrap();

    // Start VLFS
    log_info!("Initializing VLFS");
    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(device_manager.clock()));
    flash.reset().await.unwrap();
    let mut fs = VLFS::new(flash, crc);
    fs.init().await.unwrap();

    // Start GPS (provides unix time)
    log_info!("Initializing GPS");
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
    log_info!("Initializing Buzzer Queue");
    let buzzer_queue_runner = BuzzerQueueRunner::new();
    let buzzer_queue_runner_fut = buzzer_queue_runner.run(buzzer, device_manager.delay());
    let buzzer_queue = buzzer_queue_runner.get_queue();

    let services = VLSystemServices {
        fs: &fs,
        gps: &gps_location_pubsub,
        delay: device_manager.delay(),
        clock: device_manager.clock(),
        unix_clock,
        buzzer_queue,
        sys_reset: sys_reset.take().unwrap(),
    };

    let delay = device_manager.delay();
    let usb_connected = {
        log_info!("Waiting for USB connection");
        let timeout_fut = delay.delay_ms(500.0);
        let usb_wait_connection_fut = usb.wait_connection();
        match select(timeout_fut, usb_wait_connection_fut).await {
            Either::First(_) => {
                log_info!("USB not connected");
                false
            }
            Either::Second(_) => {
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

    log_info!("Initializing RPC Server");
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
    log_info!("Initializing RPC Server Done");

    let main_fut = async {
        if device_config.as_ref().is_none() {
            log_info!("No device mode file found, halting");
            claim_devices!(device_manager, indicators);
            indicators
                .run([333, 666], [0, 333, 333, 333], [0, 666, 333, 0])
                .await;
        }
        let device_config = device_config.as_ref().unwrap();

        let stop_if_usb_connected = async || {
            if usb_connected {
                log_info!("USB connected on boot, stopping");
                claim_devices!(device_manager, indicators);
                indicators.run([1000, 1000], [0, 1000, 1000, 0], []).await;
            }
        };
        log_info!("Starting in mode {:?}", device_config);
        match device_config.mode {
            DeviceModeConfig::Avionics { .. } => {
                // stop_if_usb_connected().await;
                // avionics_main(
                //     device_manager,
                //     &services,
                //     &device_config,
                //     device_serial_number,
                // )
                // .await
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
            DeviceModeConfig::GroundTestAvionics { .. } => {
                stop_if_usb_connected().await;
                ground_test_avionics(device_manager, &services, &device_config).await
            }
            DeviceModeConfig::VacuumTest => {
                stop_if_usb_connected().await;
                vacuum_test_main(device_manager, &services).await
            }
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
