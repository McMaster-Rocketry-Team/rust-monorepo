use arming_state::ArmingStateManager;
use backup_flight_core::BackupFlightCore;
use core::cell::RefCell;
use crc::CRC_16_GSM;
use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex, signal::Signal};
use flight_core_event::FlightCoreState;
use flight_core_event_channel::{FlightCoreEventChannel, FlightCoreEventChannelPublisher};
use flight_profile::{FlightProfile, PyroSelection};
use futures::join;
use imu_calibration_info::IMUCalibrationInfo;
use libm::fabsf;
use nalgebra::Vector3;
use vlfs::{Crc, FileType, Flash};

use crate::{
    avionics::{
        flight_core::{FlightCore, Variances},
        flight_core_event::FlightCoreEvent,
    },
    claim_devices,
    common::{
        can_bus::messages::ResetMessage,
        delta_logger::{
            buffered_tiered_ring_delta_logger::BufferedTieredRingDeltaLogger,
            delta_logger::UnixTimestampLog,
        },
        vl_device_manager::prelude::*,
        sensor_reading::SensorReading,
        sensor_snapshot::PartialSensorSnapshot,
        ticker::Ticker,
        vlp::packet::VLPDownlinkPacket,
    },
    vl_device_manager_type,
    driver::{
        adc::ADCData,
        barometer::BaroData,
        gps::{GPSData, GPS},
        imu::IMUData,
        indicator::Indicator,
        mag::MagData,
    },
    fixed_point_factory, pyro,
};
use crate::{common::can_bus::node_types::VOID_LAKE_NODE_TYPE, driver::can_bus::CanBusTX};
use crate::{
    common::{
        can_bus::messages::{self as can_messages},
        config_file::ConfigFile,
        delta_logger::prelude::{RingDeltaLoggerConfig, TieredRingDeltaLogger},
        device_config::DeviceConfig,
        file_types::*,
        vlp::{
            packet::{LowPowerModePacket, SoftArmPacket, VLPUplinkPacket},
            telemetry_packet::TelemetryPacketBuilder,
            uplink_client::VLPUplinkClient,
        },
    },
    driver::timestamp::BootTimestamp,
};
use self_test::{self_test, SelfTestResult};

pub mod arming_state;
pub mod backup_flight_core;
pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;
mod flight_core_event_channel;
pub mod flight_profile;
mod imu_calibration_info;
mod self_test;
pub mod vertical_speed_filter;

#[inline(never)]
pub async fn avionics_main(
    device_manager: vl_device_manager_type!(),
    services: system_services_type!(),
    config: &DeviceConfig,
    device_serial_number: &[u8; 12],
) -> ! {
    claim_devices!(device_manager, indicators);

    let flight_profile_file =
        ConfigFile::<FlightProfile, _, _>::new(services.fs, FLIGHT_PROFILE_FILE_TYPE);
    let flight_profile: FlightProfile =
        if let Some(flight_profile) = flight_profile_file.read().await {
            log_info!("Flight profile: {:?}", flight_profile);
            flight_profile
        } else {
            log_info!("No flight profile file found, halting");
            indicators
                .run([333, 666], [0, 333, 333, 333], [0, 666, 333, 0])
                .await;
            log_unreachable!();
        };

    log_info!("Running self test");
    match self_test(device_manager).await {
        SelfTestResult::Ok => {
            log_info!("Self test passed");
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(3000, 50, 150);
            services.buzzer_queue.publish(3000, 50, 150);
        }
        SelfTestResult::PartialFailed => {
            log_warn!("Self test partially failed");
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(3000, 50, 150);
            services.buzzer_queue.publish(2000, 50, 150);
        }
        SelfTestResult::Failed => {
            log_error!("Self test failed");
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(2000, 50, 150);
            services.buzzer_queue.publish(3000, 50, 150);
            services.buzzer_queue.publish(3000, 50, 150);
            indicators.run([200, 200], [], []).await;
        }
    }

    let get_logger_config = |tier_1_file_type: FileType,
                             tier_1_first_segment_seconds: u32,
                             tier_2_file_type: FileType,
                             tier_2_first_segment_seconds: u32| {
        let tier_1_config = RingDeltaLoggerConfig {
            file_type: tier_1_file_type,
            seconds_per_segment: 5 * 60,
            first_segment_seconds: tier_1_first_segment_seconds,
            segments_per_ring: 6, // 30 min
        };
        let tier_2_config = RingDeltaLoggerConfig {
            file_type: tier_2_file_type,
            seconds_per_segment: 30 * 60,
            first_segment_seconds: tier_2_first_segment_seconds,
            segments_per_ring: 10, // 5 hours
        };

        (tier_1_config, tier_2_config)
    };

    log_info!("Creating GPS logger");
    let gps_logger = BufferedTieredRingDeltaLogger::<GPSData, 40>::new();
    fixed_point_factory!(GPSFF1, f64, 99.0, 110.0, 0.5);
    fixed_point_factory!(GPSFF2, f64, 4999.0, 5010.0, 0.5);
    let gps_logger_fut = gps_logger.run(
        GPSFF1,
        GPSFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_GPS_LOGGER_TIER_1,
                25 * 1,
                AVIONICS_GPS_LOGGER_TIER_2,
                25 * 2,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    fixed_point_factory!(SensorsFF1, f64, 4.9, 7.0, 0.05);
    fixed_point_factory!(SensorsFF2, f64, 199.0, 210.0, 0.5);

    log_info!("Creating low G IMU logger");
    let low_g_imu_logger = BufferedTieredRingDeltaLogger::<IMUData, 40>::new();
    let low_g_imu_logger_fut = low_g_imu_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_LOW_G_IMU_LOGGER_TIER_1,
                25 * 3,
                AVIONICS_LOW_G_IMU_LOGGER_TIER_2,
                25 * 4,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating High G IMU logger");
    let high_g_imu_logger = BufferedTieredRingDeltaLogger::<IMUData, 40>::new();
    let high_g_imu_logger_fut = high_g_imu_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_HIGH_G_IMU_LOGGER_TIER_1,
                25 * 5,
                AVIONICS_HIGH_G_IMU_LOGGER_TIER_2,
                25 * 6,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating baro logger");
    let baro_logger = BufferedTieredRingDeltaLogger::<BaroData, 40>::new();
    let baro_logger_fut = baro_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_BARO_LOGGER_TIER_1,
                25 * 7,
                AVIONICS_BARO_LOGGER_TIER_2,
                25 * 8,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    fixed_point_factory!(MagFF1, f64, 49.9, 55.0, 0.05);
    log_info!("Creating Mag logger");
    let mag_logger = BufferedTieredRingDeltaLogger::<MagData, 40>::new();
    let mag_logger_fut = mag_logger.run(
        MagFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_MAG_LOGGER_TIER_1,
                25 * 9,
                AVIONICS_MAG_LOGGER_TIER_2,
                25 * 10,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating battery logger");
    let battery_logger = BufferedTieredRingDeltaLogger::<ADCData<Volt>, 40>::new();
    let battery_logger_fut = battery_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_BATTERY_LOGGER_TIER_1,
                25 * 11,
                AVIONICS_BATTERY_LOGGER_TIER_2,
                25 * 12,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!(
        "Loggers created, free size: {}MB",
        services.fs.free().await / 1024 / 1024
    );

    // states
    let storage_full = RefCell::new(false);
    let low_power_mode = RefCell::new(false);
    let arming_state = ArmingStateManager::<NoopRawMutex>::new();
    let arming_state_debounce_fut = arming_state.run_debounce(services.delay.clone());

    let flight_core_events = FlightCoreEventChannel::new();
    let flight_core: RefCell<Option<FlightCore<FlightCoreEventChannelPublisher>>> =
        RefCell::new(None);
    let backup_flight_core: RefCell<Option<BackupFlightCore<FlightCoreEventChannelPublisher>>> =
        RefCell::new(None);

    let vertical_calibration_in_progress = RefCell::new(false);
    let imu_baro_signal = Signal::<
        NoopRawMutex,
        (
            SensorReading<BootTimestamp, IMUData>,
            SensorReading<BootTimestamp, IMUData>,
            SensorReading<BootTimestamp, BaroData>,
        ),
    >::new();

    let imu_config_file = ConfigFile::<IMUCalibrationInfo, _, _>::new(
        services.fs,
        UPRIGHT_VECTOR_AND_GYRO_OFFSET_FILE_TYPE,
    );
    let imu_config = RefCell::new(imu_config_file.read().await);

    log_info!("Claiming devices");
    claim_devices!(
        device_manager,
        arming_switch,
        low_g_imu,
        high_g_imu,
        barometer,
        mag,
        batt_voltmeter,
        lora,
        camera,
        can_bus
    );
    log_info!("Devices claimed");

    let mut can_bus = can_bus.take().unwrap();
    can_bus.reset().await.unwrap();
    let crc = crc::Crc::<u16>::new(&CRC_16_GSM);
    can_bus.configure_self_node(
        VOID_LAKE_NODE_TYPE,
        crc.checksum(device_serial_number) & 0xFFF,
    );
    drop(crc);
    let (can_tx, _) = can_bus.split();

    let can_tx = Mutex::<NoopRawMutex, _>::new(can_tx);

    let can_tx_avionics_status_fut = async {
        let mut ticker = Ticker::every(services.clock(), services.delay(), 2000.0);
        loop {
            let message = can_messages::AvionicsStatusMessage {
                low_power: *low_power_mode.borrow(),
                armed: arming_state.is_armed(),
            };
            let mut can_tx = can_tx.lock().await;
            can_tx.send(&message, 3).await.ok();
            drop(can_tx);

            ticker.next().await;
        }
    };

    let can_tx_unix_time_fut = async {
        let mut unix_clock_sub = services.unix_clock.subscribe_unix_clock_update();
        loop {
            let unix_timestamp = unix_clock_sub.next_message_pure().await;
            let mut can_tx = can_tx.lock().await;
            let message = can_messages::UnixTimeMessage {
                timestamp: (unix_timestamp as u64).into(),
            };
            can_tx.send(&message, 2).await.ok();
            drop(can_tx);
        }
    };

    let indicators_fut = async {
        let wait_gps_fut = services.unix_clock.wait_until_ready();
        let wait_gps_indicator_fut = indicators.run([], [], [250, 250]);
        select(wait_gps_fut, wait_gps_indicator_fut).await;
        indicators.run([], [50, 950], []).await;
    };

    let telemetry_packet_builder = TelemetryPacketBuilder::new(services.unix_clock());
    let vlp = VLPUplinkClient::new();
    let vlp_tx_fut = async {
        let mut update_ticker = Ticker::every(services.clock(), services.delay(), 1000.0);
        loop {
            update_ticker.next().await;

            let free = services.fs.free().await;
            // log_info!("Free space: {}MB", free / 1024 / 1024);
            telemetry_packet_builder.update(|b| {
                b.disk_free_space = free;
            });
            let packet = telemetry_packet_builder.create_packet();
            vlp.send(VLPDownlinkPacket::TelemetryPacket(packet));
        }
    };
    let vlp_rx_fut = async {
        loop {
            let (packet, _) = vlp.wait_receive().await;
            low_power_mode.replace(false);
            log_info!("Received packet: {:?}", packet);
            match packet {
                VLPUplinkPacket::VerticalCalibrationPacket(_) => {
                    log_info!("Vertical calibration");
                    if !arming_state.is_armed() {
                        vertical_calibration_in_progress.replace(true);
                        let mut acc_sum = Vector3::<f32>::zeros();
                        let mut gyro_sum = Vector3::<f32>::zeros();
                        for _ in 0..100 {
                            let (low_g_imu_reading, _, _) = imu_baro_signal.wait().await;
                            acc_sum += Vector3::from(low_g_imu_reading.data.acc);
                            gyro_sum += Vector3::from(low_g_imu_reading.data.gyro);
                        }
                        acc_sum /= 100.0;
                        gyro_sum /= 100.0;

                        let new_imu_config = IMUCalibrationInfo {
                            gyro_offset: (-gyro_sum).into(),
                            up_right_vector: acc_sum.into(),
                        };
                        imu_config_file.write(&new_imu_config).await.ok();
                        imu_config.replace(Some(new_imu_config));
                        services.buzzer_queue.publish(2000, 50, 100);
                        services.buzzer_queue.publish(2000, 50, 100);
                        vertical_calibration_in_progress.replace(false);
                    }
                }
                VLPUplinkPacket::SoftArmPacket(SoftArmPacket { armed, .. }) => {
                    arming_state.set_software_armed(armed);
                    telemetry_packet_builder.update(|b| {
                        b.software_armed = armed;
                    });
                }
                VLPUplinkPacket::LowPowerModePacket(LowPowerModePacket { enabled, .. }) => {
                    low_power_mode.replace(enabled);
                    if !enabled {
                        arming_state.set_software_armed(false);
                        telemetry_packet_builder.update(|b| {
                            b.software_armed = false;
                        });
                    }
                }
                VLPUplinkPacket::ResetPacket(_) => {
                    let mut can_tx = can_tx.lock().await;
                    can_tx.send(&ResetMessage {}, 7).await.ok();
                    drop(can_tx);
                    services.delay().delay_ms(100.0).await;
                    services.reset();
                }
                VLPUplinkPacket::DeleteLogsPacket(_) => {
                    services
                        .fs
                        .remove_files(|file_entry| {
                            let typ = file_entry.typ;
                            return typ == BENCHMARK_FILE_TYPE
                                || typ == AVIONICS_SENSORS_FILE_TYPE
                                || typ == AVIONICS_LOG_FILE_TYPE
                                || typ == GROUND_TEST_LOG_FILE_TYPE
                                || typ == AVIONICS_GPS_LOGGER_TIER_1
                                || typ == AVIONICS_GPS_LOGGER_TIER_2
                                || typ == AVIONICS_LOW_G_IMU_LOGGER_TIER_1
                                || typ == AVIONICS_LOW_G_IMU_LOGGER_TIER_2
                                || typ == AVIONICS_HIGH_G_IMU_LOGGER_TIER_1
                                || typ == AVIONICS_HIGH_G_IMU_LOGGER_TIER_2
                                || typ == AVIONICS_BARO_LOGGER_TIER_1
                                || typ == AVIONICS_BARO_LOGGER_TIER_2
                                || typ == AVIONICS_MAG_LOGGER_TIER_1
                                || typ == AVIONICS_MAG_LOGGER_TIER_2
                                || typ == AVIONICS_BATTERY_LOGGER_TIER_1
                                || typ == AVIONICS_BATTERY_LOGGER_TIER_2;
                        })
                        .await
                        .ok();
                }
                VLPUplinkPacket::GroundTestDeployPacket(_) => {
                    // noop
                }
            }
        }
    };
    let vlp_fut = async {
        if let Some(lora) = lora.as_mut() {
            vlp.run(
                services.delay(),
                lora,
                &config.lora,
                services.unix_clock(),
                &config.lora_key,
            )
            .await;
        }
    };

    let hardware_arming_fut = async {
        let mut hardware_armed = arming_switch.read_arming().await.unwrap();
        loop {
            arming_state.set_hardware_armed(hardware_armed);
            telemetry_packet_builder.update(|b| {
                b.hardware_armed = hardware_armed;
            });
            hardware_armed = arming_switch.wait_arming_change().await.unwrap();
        }
    };

    let hardware_arming_beep_fut = async {
        let mut sub = arming_state.subscriber();
        let mut hardware_armed_debounced = arming_state.is_armed();
        if hardware_armed_debounced {
            services.buzzer_queue.publish(2000, 700, 300);
            services.buzzer_queue.publish(3000, 700, 300);
        }

        loop {
            let new_hardware_armed = sub.next_message_pure().await.hardware_armed;
            if !hardware_armed_debounced && new_hardware_armed {
                services.buzzer_queue.publish(2000, 700, 300);
                services.buzzer_queue.publish(3000, 700, 300);
            } else if hardware_armed_debounced && !new_hardware_armed {
                services.buzzer_queue.publish(3000, 700, 300);
                services.buzzer_queue.publish(2000, 700, 300);
            }
            hardware_armed_debounced = new_hardware_armed;
        }
    };

    let pyro_main_cont_fut = async {
        let mut cont = pyro!(
            device_manager,
            flight_profile.main_pyro,
            pyro_cont.read_continuity().await.unwrap()
        );

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_main_continuity = cont;
            });
            cont = pyro!(
                device_manager,
                flight_profile.main_pyro,
                pyro_cont.wait_continuity_change().await.unwrap()
            );
        }
    };

    let pyro_drogue_cont_fut = async {
        let mut cont = pyro!(
            device_manager,
            flight_profile.drogue_pyro,
            pyro_cont.read_continuity().await.unwrap()
        );

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_drogue_continuity = cont;
            });
            cont = pyro!(
                device_manager,
                flight_profile.drogue_pyro,
                pyro_cont.wait_continuity_change().await.unwrap()
            );
        }
    };

    let mut imu_baro_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let imu_baro_fut = async {
        loop {
            imu_baro_ticker.next().await;
            if *low_power_mode.borrow() {
                continue;
            }

            let (low_g_imu_result, high_g_imu_result, baro_result) =
                join!(low_g_imu.read(), high_g_imu.read(), barometer.read(),);
            let low_g_imu_reading = low_g_imu_result.unwrap();
            let high_g_imu_reading = high_g_imu_result.unwrap();
            let baro_reading = baro_result.unwrap();

            if !*storage_full.borrow() {
                low_g_imu_logger.log(low_g_imu_reading.clone());
                high_g_imu_logger.log(high_g_imu_reading.clone());
                baro_logger.log(baro_reading.clone());
            }

            imu_baro_signal.signal((low_g_imu_reading, high_g_imu_reading, baro_reading));
        }
    };

    let mut gps_sub = services.gps.subscriber().unwrap();
    let gps_fut = async {
        loop {
            let gps_location = gps_sub.next_message_pure().await;
            if !*storage_full.borrow() {
                gps_logger.log(gps_location.clone());
            }
            telemetry_packet_builder.update(|b| {
                b.gps_location = Some(gps_location.data);
            });
        }
    };

    let mut mag_ticker = Ticker::every(services.clock(), services.delay(), 50.0);
    let mag_fut = async {
        loop {
            mag_ticker.next().await;
            if *low_power_mode.borrow() {
                continue;
            }
            let mag_reading = mag.read().await.unwrap();

            if !*storage_full.borrow() {
                mag_logger.log(mag_reading.clone());
            }
        }
    };

    let mut batt_volt_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let bat_fut = async {
        loop {
            let battery_v = batt_voltmeter.read().await.unwrap();
            if !*storage_full.borrow() {
                battery_logger.log(battery_v.clone());
            }
            telemetry_packet_builder.update(|b| {
                b.battery_v = battery_v.data.value;
            });
            batt_volt_ticker.next().await;
        }
    };

    let loggers_unix_time_log_fut = async {
        let mut unix_clock_sub = services.unix_clock.subscribe_unix_clock_update();
        loop {
            let unix_timestamp = unix_clock_sub.next_message_pure().await;
            let boot_timestamp = services.clock.now_ms();
            let log = UnixTimestampLog {
                unix_timestamp,
                boot_timestamp,
            };
            gps_logger.log_unix_time(log.clone());
            low_g_imu_logger.log_unix_time(log.clone());
            high_g_imu_logger.log_unix_time(log.clone());
            baro_logger.log_unix_time(log.clone());
            mag_logger.log_unix_time(log.clone());
            battery_logger.log_unix_time(log.clone());
        }
    };

    let setup_flight_core_fut = async {
        let mut arming_state_sub = arming_state.subscriber();
        loop {
            let armed = arming_state_sub.next_message_pure().await.is_armed();
            let flight_core_initialized = flight_core.borrow().is_some();
            if armed && !flight_core_initialized {
                if let Some(imu_config) = imu_config.borrow().clone() {
                    flight_core.replace(Some(FlightCore::new(
                        flight_profile.clone(),
                        flight_core_events.publisher(false),
                        imu_config.up_right_vector.into(),
                        Variances::default(),
                    )));
                }
                backup_flight_core.replace(Some(BackupFlightCore::new(
                    flight_profile.clone(),
                    flight_core_events.publisher(true),
                )));
            } else if !armed && flight_core_initialized {
                flight_core.take();
                backup_flight_core.take();
            }
        }
    };

    let flight_core_tick_fut = async {
        loop {
            if *vertical_calibration_in_progress.borrow() {
                services.delay.delay_ms(100.0).await;
                continue;
            }
            let (low_g_imu_reading, high_g_imu_reading, baro_reading) =
                imu_baro_signal.wait().await;

            let mut combined_imu_reading = low_g_imu_reading.clone();
            if fabsf(combined_imu_reading.data.acc[0]) > 15.0 * 9.81
                || fabsf(combined_imu_reading.data.acc[1]) > 15.0 * 9.81
                || fabsf(combined_imu_reading.data.acc[2]) > 15.0 * 9.81
            {
                combined_imu_reading.data.acc = high_g_imu_reading.data.acc;
            }

            if let Some(backup_flight_core) = backup_flight_core.borrow_mut().as_mut() {
                backup_flight_core.tick(&baro_reading)
            }
            if let Some(flight_core) = flight_core.borrow_mut().as_mut() {
                flight_core.tick(PartialSensorSnapshot {
                    timestamp: combined_imu_reading.timestamp,
                    imu_reading: combined_imu_reading,
                    baro_reading: Some(baro_reading),
                })
            }
        }
    };

    let flight_core_event_consumer = async {
        let mut sub = flight_core_events.subscriber();

        loop {
            let (is_backup, event) = sub.next_message_pure().await;
            match event {
                FlightCoreEvent::CriticalError => {
                    services.reset();
                }
                FlightCoreEvent::DidNotReachMinApogee => {
                    // noop
                }
                FlightCoreEvent::ChangeState(new_state) => {
                    telemetry_packet_builder.update(|s| {
                        if is_backup {
                            s.backup_flight_core_state = new_state;
                        } else {
                            s.flight_core_state = new_state;
                        }
                    });
                }
                FlightCoreEvent::ChangeAltitude(new_altitude) => {
                    telemetry_packet_builder.update(|s| {
                        if is_backup {
                            s.backup_altitude = new_altitude;
                        } else {
                            s.altitude = new_altitude;
                        }
                    });
                }
                FlightCoreEvent::ChangeAirSpeed(new_speed) => {
                    telemetry_packet_builder.update(|s| {
                        if is_backup {
                            s.backup_air_speed = new_speed;
                        } else {
                            s.air_speed = new_speed;
                        }
                    });
                }
            }
        }
    };

    let can_tx_flight_event_fut = async {
        let mut sub = flight_core_events.subscriber();

        let can_send_flight_event = async |event: can_messages::FlightEvent| {
            let message = can_messages::FlightEventMessage {
                timestamp: (services.unix_clock.now_ms() as u64).into(),
                event,
            };
            let mut can_tx = can_tx.lock().await;
            can_tx.send(&message, 7).await.ok();
            drop(can_tx);
        };

        loop {
            // FIXME duplicate events, events may not be in order
            if let (_, FlightCoreEvent::ChangeState(state)) = sub.next_message_pure().await {
                match state {
                    FlightCoreState::PowerAscend => {
                        can_send_flight_event(can_messages::FlightEvent::Ignition).await;
                    }
                    FlightCoreState::Coast => {
                        can_send_flight_event(can_messages::FlightEvent::Coast).await;
                    }
                    FlightCoreState::Descent => {
                        can_send_flight_event(can_messages::FlightEvent::Apogee).await;
                    }
                    FlightCoreState::Landed => {
                        can_send_flight_event(can_messages::FlightEvent::Landed).await;
                    }
                    _ => {}
                }
            }
        }
    };

    let pyro_main_ctrl_fut = async {
        let mut sub = flight_core_events.subscriber();

        // only react to backup flight core for now
        loop {
            if matches!(
                sub.next_message_pure().await,
                (
                    true,
                    FlightCoreEvent::ChangeState(FlightCoreState::MainChuteDeployed)
                )
            ) {
                pyro!(
                    device_manager,
                    flight_profile.main_pyro,
                    pyro_ctrl.set_enable(true).await.ok()
                );
                services.delay.delay_ms(3000.0).await;
                pyro!(
                    device_manager,
                    flight_profile.main_pyro,
                    pyro_ctrl.set_enable(false).await.ok()
                );
                break;
            }
        }
    };

    let pyro_drogue_ctrl_fut = async {
        let mut sub = flight_core_events.subscriber();

        // only react to backup flight core for now
        loop {
            if matches!(
                sub.next_message_pure().await,
                (
                    true,
                    FlightCoreEvent::ChangeState(FlightCoreState::DrogueChuteDeployed)
                )
            ) {
                pyro!(
                    device_manager,
                    flight_profile.drogue_pyro,
                    pyro_ctrl.set_enable(true).await.ok()
                );
                services.delay.delay_ms(3000.0).await;
                pyro!(
                    device_manager,
                    flight_profile.drogue_pyro,
                    pyro_ctrl.set_enable(false).await.ok()
                );
                break;
            }
        }
    };

    let camera_ctrl_fut = async {
        let mut sub = flight_core_events.subscriber();

        loop {
            if let (_, FlightCoreEvent::ChangeState(state)) = sub.next_message_pure().await {
                match state {
                    FlightCoreState::Armed => {
                        camera.set_recording(true).await.ok();
                    }
                    FlightCoreState::DisArmed => {
                        camera.set_recording(false).await.ok();
                    }
                    FlightCoreState::Landed => {
                        services.delay().delay_ms(1000.0 * 60.0).await;
                        camera.set_recording(false).await.ok();
                    }
                    _ => {}
                }
            }
        }
    };

    let mut storage_full_detection_ticker =
        Ticker::every(services.clock(), services.delay(), 1000.0);
    let storage_full_detection_fut = async {
        loop {
            storage_full_detection_ticker.next().await;
            let free = services.fs.free().await;
            storage_full.replace(free < 1024 * 1024);
        }
    };

    join!(
        gps_logger_fut,
        low_g_imu_logger_fut,
        high_g_imu_logger_fut,
        baro_logger_fut,
        mag_logger_fut,
        battery_logger_fut,
        vlp_tx_fut,
        vlp_rx_fut,
        vlp_fut,
        hardware_arming_fut,
        hardware_arming_beep_fut,
        setup_flight_core_fut,
        pyro_main_cont_fut,
        pyro_drogue_cont_fut,
        imu_baro_fut,
        gps_fut,
        mag_fut,
        bat_fut,
        flight_core_tick_fut,
        pyro_main_ctrl_fut,
        pyro_drogue_ctrl_fut,
        flight_core_event_consumer,
        can_tx_flight_event_fut,
        camera_ctrl_fut,
        can_tx_avionics_status_fut,
        can_tx_unix_time_fut,
        indicators_fut,
        storage_full_detection_fut,
        arming_state_debounce_fut,
        loggers_unix_time_log_fut,
    );
    log_unreachable!();
}
