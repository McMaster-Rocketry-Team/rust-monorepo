use core::cell::RefCell;
use crc::CRC_16_GSM;
use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex},
    channel::{Channel, Sender},
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
    signal::Signal,
};
use flight_profile::{FlightProfile, PyroSelection};
use futures::join;
use imu_calibration_info::IMUCalibrationInfo;
use nalgebra::Vector3;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::{Crc, FileType, Flash};

use crate::{
    avionics::{
        flight_core::{FlightCore, Variances},
        flight_core_event::FlightCoreEvent,
    },
    claim_devices,
    common::{
        delta_logger::buffered_tiered_ring_delta_logger::BufferedTieredRingDeltaLogger,
        device_manager::prelude::*, sensor_reading::SensorReading,
        sensor_snapshot::PartialSensorSnapshot, ticker::Ticker, vlp::packet::VLPDownlinkPacket,
    },
    device_manager_type,
    driver::{
        adc::ADCData,
        barometer::BaroData,
        debugger::DebuggerTargetEvent,
        gps::{GPSData, GPS},
        imu::IMUData,
        indicator::Indicator,
        mag::MagData,
    },
    fixed_point_factory2,
};
use crate::{common::can_bus::node_types::VOID_LAKE_NODE_TYPE, driver::can_bus::CanBusTX};
use crate::{
    common::{
        can_bus::messages::{
            AvionicsStatusMessage, FlightEvent, FlightEventMessage, UnixTimeMessage,
        },
        config_file::ConfigFile,
        delta_logger::prelude::{RingDeltaLoggerConfig, TieredRingDeltaLogger},
        device_config::{DeviceConfig, DeviceModeConfig},
        file_types::*,
        vlp::{
            packet::{LowPowerModePacket, SoftArmPacket, VLPUplinkPacket},
            telemetry_packet::{FlightCoreStateTelemetry, TelemetryPacketBuilder},
            uplink_client::VLPUplinkClient,
        },
    },
    driver::timestamp::{BootTimestamp, UnixTimestamp},
};
use self_test::{self_test, SelfTestResult};

pub mod avionics_state;
pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;
pub mod flight_profile;
mod imu_calibration_info;
mod self_test;

#[inline(never)]
pub async fn avionics_main(
    device_manager: device_manager_type!(),
    services: system_services_type!(),
    config: &DeviceConfig,
    device_serial_number: &[u8; 12],
) -> ! {
    let lora_key = if let DeviceModeConfig::Avionics { lora_key } = &config.mode {
        lora_key
    } else {
        log_unreachable!()
    };

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
                .await
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

    let (mut pyro_main_cont, mut pyro_main_ctrl) = match flight_profile.main_pyro {
        PyroSelection::Pyro1 => {
            claim_devices!(device_manager, pyro1_cont, pyro1_ctrl);
            (pyro1_cont, pyro1_ctrl)
        }
        PyroSelection::Pyro2 => {
            claim_devices!(device_manager, pyro2_cont, pyro2_ctrl);
            (pyro2_cont, pyro2_ctrl)
        }
        PyroSelection::Pyro3 => {
            todo!();
            // claim_devices!(device_manager, pyro3_cont, pyro3_ctrl);
            // (pyro3_cont.unwrap(), pyro3_ctrl.unwrap())
        }
    };
    let (mut pyro_drogue_cont, mut pyro_drouge_ctrl) = match flight_profile.drogue_pyro {
        PyroSelection::Pyro1 => {
            claim_devices!(device_manager, pyro1_cont, pyro1_ctrl);
            (pyro1_cont, pyro1_ctrl)
        }
        PyroSelection::Pyro2 => {
            claim_devices!(device_manager, pyro2_cont, pyro2_ctrl);
            (pyro2_cont, pyro2_ctrl)
        }
        PyroSelection::Pyro3 => {
            todo!();
            // claim_devices!(device_manager, pyro3_cont, pyro3_ctrl);
            // (pyro3_cont, pyro3_ctrl)
        }
    };

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
    let gps_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, GPSData, 40>::new();
    fixed_point_factory2!(GPSFF1, f64, 99.0, 110.0, 0.5);
    fixed_point_factory2!(GPSFF2, f64, 4999.0, 5010.0, 0.5);
    let gps_logger_fut = gps_logger.run(
        GPSFF1,
        GPSFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(AVIONICS_GPS_LOGGER_TIER_1, 0, AVIONICS_GPS_LOGGER_TIER_2, 0),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    fixed_point_factory2!(SensorsFF1, f64, 4.9, 7.0, 0.05);
    fixed_point_factory2!(SensorsFF2, f64, 199.0, 210.0, 0.5);

    log_info!("Creating low G IMU logger");
    let low_g_imu_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, IMUData, 40>::new();
    let low_g_imu_logger_fut = low_g_imu_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_LOW_G_IMU_LOGGER_TIER_1,
                0,
                AVIONICS_LOW_G_IMU_LOGGER_TIER_2,
                0,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating High G IMU logger");
    let high_g_imu_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, IMUData, 40>::new();
    let high_g_imu_logger_fut = high_g_imu_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_HIGH_G_IMU_LOGGER_TIER_1,
                0,
                AVIONICS_HIGH_G_IMU_LOGGER_TIER_2,
                0,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating baro logger");
    let baro_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, BaroData, 40>::new();
    let baro_logger_fut = baro_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_BARO_LOGGER_TIER_1,
                0,
                AVIONICS_BARO_LOGGER_TIER_2,
                0,
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    fixed_point_factory2!(MagFF1, f64, 49.9, 55.0, 0.05);
    log_info!("Creating Mag logger");
    let mag_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, MagData, 40>::new();
    let mag_logger_fut = mag_logger.run(
        MagFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(AVIONICS_MAG_LOGGER_TIER_1, 0, AVIONICS_MAG_LOGGER_TIER_2, 0),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    log_info!("Creating battery logger");
    let battery_logger = BufferedTieredRingDeltaLogger::<UnixTimestamp, ADCData<Volt>, 40>::new();
    let battery_logger_fut = battery_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            get_logger_config(
                AVIONICS_BATTERY_LOGGER_TIER_1,
                0,
                AVIONICS_BATTERY_LOGGER_TIER_2,
                0,
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
    let storage_full = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let low_power_mode = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let is_low_power_mode = || low_power_mode.lock(|s| *s.borrow());
    let arming_state = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(ArmingState {
        hardware_armed: false,
        software_armed: false,
    }));
    let arming_changed_signal = Signal::<NoopRawMutex, ()>::new();
    let flight_core: BlockingMutex<
        NoopRawMutex,
        RefCell<Option<FlightCore<Sender<NoopRawMutex, FlightCoreEvent, 3>>>>,
    > = BlockingMutex::new(RefCell::new(None));
    let flight_core_events = Channel::<NoopRawMutex, FlightCoreEvent, 3>::new();
    let flight_core_state_pub_sub =
        PubSubChannel::<NoopRawMutex, FlightCoreStateTelemetry, 2, 3, 1>::new();

    let vertical_calibration_in_progress =
        BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let low_g_imu_signal = Signal::<NoopRawMutex, SensorReading<BootTimestamp, IMUData>>::new();
    let high_g_imu_signal = Signal::<NoopRawMutex, SensorReading<BootTimestamp, IMUData>>::new();
    let baro_signal = Signal::<NoopRawMutex, SensorReading<BootTimestamp, BaroData>>::new();
    let pyro_main_fire_signal = Signal::<NoopRawMutex, ()>::new();
    let pyro_drouge_fire_signal = Signal::<NoopRawMutex, ()>::new();

    let imu_config_file = ConfigFile::<IMUCalibrationInfo, _, _>::new(
        services.fs,
        UPRIGHT_VECTOR_AND_GYRO_OFFSET_FILE_TYPE,
    );
    let imu_config =
        BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(imu_config_file.read().await));

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
        can_bus,
        sys_reset
    );
    log_info!("Devices claimed");

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
            let message = AvionicsStatusMessage {
                low_power: is_low_power_mode(),
                armed: arming_state.lock(|s| (*s.borrow()).is_armed()),
            };
            let mut can_tx = can_tx.lock().await;
            can_tx.send(&message, 3).await.ok();
            drop(can_tx);

            ticker.next().await;
        }
    };

    let can_tx_unix_time_fut = async {
        let mut ticker = Ticker::every(services.clock(), services.delay(), 1000.0);
        services.unix_clock.wait_until_ready().await;
        loop {
            let mut can_tx = can_tx.lock().await;
            let timestamp = services.unix_clock.now_ms() as u64;
            let message = UnixTimeMessage {
                timestamp: timestamp.into(),
            };
            can_tx.send(&message, 2).await.ok();
            drop(can_tx);

            ticker.next().await;
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
    // &config.lora,
    // services.unix_clock(),
    // services.delay(),
    // lora_key,
    let vlp_tx_fut = async {
        // Wait 1 sec for all the fields to be populated
        services.delay.delay_ms(1000.0).await;

        let mut update_ticker = Ticker::every(services.clock(), services.delay(), 1000.0);
        loop {
            let free = services.fs.free().await;
            // log_info!("Free space: {}MB", free / 1024 / 1024);
            telemetry_packet_builder.update(|b| {
                b.disk_free_space = free;
            });
            let packet = telemetry_packet_builder.create_packet();
            vlp.send(VLPDownlinkPacket::TelemetryPacket(packet));
            update_ticker.next().await;
        }
    };
    let vlp_rx_fut = async {
        loop {
            let (packet, status) = vlp.wait_receive().await;
            low_power_mode.lock(|s| *s.borrow_mut() = false);
            log_info!("Received packet: {:?}", packet);
            match packet {
                VLPUplinkPacket::VerticalCalibrationPacket(_) => {
                    log_info!("Vertical calibration");
                    if services.unix_clock.ready()
                        && !arming_state.lock(|s| (*s.borrow()).is_armed())
                    {
                        vertical_calibration_in_progress.lock(|s| *s.borrow_mut() = true);
                        let mut acc_sum = Vector3::<f32>::zeros();
                        let mut gyro_sum = Vector3::<f32>::zeros();
                        for _ in 0..100 {
                            let reading = low_g_imu_signal.wait().await;
                            acc_sum += Vector3::from(reading.data.acc);
                            gyro_sum += Vector3::from(reading.data.gyro);
                        }
                        acc_sum /= 100.0;
                        gyro_sum /= 100.0;

                        let new_imu_config = IMUCalibrationInfo {
                            gyro_offset: (-gyro_sum).into(),
                            up_right_vector: acc_sum.into(),
                        };
                        imu_config_file.write(&new_imu_config).await.ok();
                        imu_config.lock(|s| s.borrow_mut().replace(new_imu_config));
                        services.buzzer_queue.publish(2000, 50, 100);
                        services.buzzer_queue.publish(2000, 50, 100);
                        vertical_calibration_in_progress.lock(|s| *s.borrow_mut() = false);
                    }
                }
                VLPUplinkPacket::SoftArmPacket(SoftArmPacket { armed, .. }) => {
                    arming_state.lock(|s| s.borrow_mut().software_armed = armed);
                    arming_changed_signal.signal(());
                    telemetry_packet_builder.update(|b| {
                        b.software_armed = armed;
                    });
                }
                VLPUplinkPacket::LowPowerModePacket(LowPowerModePacket { enabled, .. }) => {
                    low_power_mode.lock(|s| *s.borrow_mut() = enabled);
                    if !enabled {
                        arming_state.lock(|s| s.borrow_mut().software_armed = false);
                    }
                }
                VLPUplinkPacket::ResetPacket(_) => {
                    sys_reset.reset();
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
            }
        }
    };
    let vlp_fut = vlp.run(
        services.delay(),
        &mut lora,
        &config.lora,
        services.unix_clock(),
        lora_key,
    );

    let hardware_arming_fut = async {
        let mut hardware_armed = arming_switch.read_arming().await.unwrap();
        if hardware_armed {
            services.buzzer_queue.publish(2000, 700, 300);
            services.buzzer_queue.publish(3000, 700, 300);
        }
        loop {
            arming_state.lock(|s| {
                s.borrow_mut().hardware_armed = hardware_armed;
            });
            telemetry_packet_builder.update(|b| {
                b.hardware_armed = hardware_armed;
            });
            arming_changed_signal.signal(());
            hardware_armed = arming_switch.wait_arming_change().await.unwrap();
            if hardware_armed {
                services.buzzer_queue.publish(2000, 700, 300);
                services.buzzer_queue.publish(3000, 700, 300);
            } else {
                services.buzzer_queue.publish(3000, 700, 300);
                services.buzzer_queue.publish(2000, 700, 300);
            }
        }
    };

    let setup_flight_core_fut = async {
        loop {
            arming_changed_signal.wait().await;
            let armed = arming_state.lock(|s| (*s.borrow()).is_armed());
            let flight_core_initialized = flight_core.lock(|s| s.borrow().is_some());
            if armed && !flight_core_initialized {
                if let Some(imu_config) = imu_config.lock(|s| s.borrow().clone()) {
                    flight_core.lock(|s| {
                        let mut s = s.borrow_mut();
                        s.replace(FlightCore::new(
                            flight_profile.clone(),
                            flight_core_events.sender(),
                            imu_config.up_right_vector.into(),
                            Variances::default(),
                        ));
                    })
                }
            } else if !armed && flight_core_initialized {
                flight_core.lock(|s| s.take());
                flight_core_state_pub_sub.publish_immediate(FlightCoreStateTelemetry::DisArmed);
            }
        }
    };

    let pyro_main_cont_fut = async {
        let mut cont = pyro_main_cont.read_continuity().await.unwrap();

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_main_continuity = cont;
            });
            cont = pyro_main_cont.wait_continuity_change().await.unwrap();
        }
    };

    let pyro_drogue_cont_fut = async {
        let mut cont = pyro_drogue_cont.read_continuity().await.unwrap();

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_drogue_continuity = cont;
            });
            cont = pyro_drogue_cont.wait_continuity_change().await.unwrap();
        }
    };

    let mut low_g_imu_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let low_g_imu_fut = async {
        services.unix_clock.wait_until_ready().await;
        loop {
            low_g_imu_ticker.next().await;
            if is_low_power_mode() {
                continue;
            }
            let imu_reading = low_g_imu.read().await.unwrap();
            low_g_imu_signal.signal(imu_reading.clone());
            low_g_imu_logger.log(imu_reading.to_unix_timestamp(&services.unix_clock()));
        }
    };

    let mut high_g_imu_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let high_g_imu_fut = async {
        services.unix_clock.wait_until_ready().await;
        loop {
            high_g_imu_ticker.next().await;
            if is_low_power_mode() {
                continue;
            }
            let imu_reading = high_g_imu.read().await.unwrap();
            high_g_imu_signal.signal(imu_reading.clone());
            high_g_imu_logger.log(imu_reading.to_unix_timestamp(&services.unix_clock()));
        }
    };

    let mut baro_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let baro_fut = async {
        services.unix_clock.wait_until_ready().await;
        loop {
            baro_ticker.next().await;
            if is_low_power_mode() {
                continue;
            }

            let baro_reading = barometer.read().await.unwrap();
            baro_signal.signal(baro_reading.clone());
            telemetry_packet_builder.update(|b| {
                b.temperature = baro_reading.data.temperature;
            });
            baro_logger.log(baro_reading.to_unix_timestamp(&services.unix_clock()));
        }
    };

    let mut gps_sub = services.gps.subscriber().unwrap();
    let gps_fut = async {
        loop {
            let gps_location = gps_sub.next_message_pure().await;
            gps_logger.log(gps_location.to_unix_timestamp(&services.unix_clock()));
            telemetry_packet_builder.update(|b| {
                b.gps_location = Some(gps_location.data);
            });
        }
    };

    let mut mag_ticker = Ticker::every(services.clock(), services.delay(), 50.0);
    let mag_fut = async {
        services.unix_clock.wait_until_ready().await;
        loop {
            mag_ticker.next().await;
            if is_low_power_mode() {
                continue;
            }
            let mag_reading = mag.read().await.unwrap();
            mag_logger.log(mag_reading.to_unix_timestamp(&services.unix_clock()));
        }
    };

    let mut batt_volt_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let bat_fut = async {
        loop {
            let battery_v = batt_voltmeter.read().await.unwrap();
            if services.unix_clock.ready() {
                battery_logger.log(battery_v.clone().to_unix_timestamp(&services.unix_clock()));
            }
            telemetry_packet_builder.update(|b| {
                b.battery_v = battery_v.data.value;
            });
            batt_volt_ticker.next().await;
        }
    };

    let flight_core_tick_fut = async {
        loop {
            if vertical_calibration_in_progress.lock(|s| *s.borrow()) {
                services.delay.delay_ms(100.0).await;
                continue;
            }
            let low_g_imu_reading = low_g_imu_signal.wait().await;
            let high_g_imu_reading = high_g_imu_signal.wait().await;
            let baro_reading = baro_signal.wait().await;

            let mut combined_imu_reading = low_g_imu_reading.clone();
            if Vector3::from(combined_imu_reading.data.acc).magnitude() > 15.0 * 9.81 {
                combined_imu_reading.data.acc = high_g_imu_reading.data.acc;
            }

            flight_core.lock(|flight_core| {
                let mut flight_core = flight_core.borrow_mut();
                if let Some(flight_core) = flight_core.as_mut() {
                    flight_core.tick(PartialSensorSnapshot {
                        timestamp: combined_imu_reading.timestamp,
                        imu_reading: combined_imu_reading,
                        baro_reading: Some(baro_reading),
                    })
                }
            })
        }
    };

    let flight_core_event_consumer = async {
        let receiver = flight_core_events.receiver();

        let debugger = device_manager.debugger.clone();
        loop {
            let event = receiver.receive().await;
            match event {
                FlightCoreEvent::ChangeAltitude(_) => {}
                _ => {
                    debugger.dispatch(DebuggerTargetEvent::FlightCoreEvent(event));
                }
            }
            match event {
                FlightCoreEvent::CriticalError => {
                    claim_devices!(device_manager, sys_reset);
                    sys_reset.reset();
                }
                FlightCoreEvent::Ignition => {
                    // noop
                }
                FlightCoreEvent::Apogee => {
                    // noop
                }
                FlightCoreEvent::DeployMain => {
                    pyro_main_fire_signal.signal(());
                }
                FlightCoreEvent::DeployDrogue => {
                    pyro_drouge_fire_signal.signal(());
                }
                FlightCoreEvent::Landed => {
                    // noop
                }
                FlightCoreEvent::DidNotReachMinApogee => {
                    // noop
                }
                FlightCoreEvent::ChangeState(new_state) => {
                    flight_core_state_pub_sub.publish_immediate(new_state);
                    telemetry_packet_builder.update(|s| {
                        s.flight_core_state = new_state;
                    });
                }
                FlightCoreEvent::ChangeAltitude(new_altitude) => {
                    telemetry_packet_builder.update(|s| {
                        s.altitude = new_altitude;
                    });
                }
                FlightCoreEvent::ChangeSpeed(new_speed) => {
                    telemetry_packet_builder.update(|s| {
                        // TODO is new_speed vertical speed?
                        s.vertical_speed = new_speed;
                    });
                }
            }
        }
    };

    let flight_core_state_sub_fut = async {
        let mut sub = flight_core_state_pub_sub.subscriber().unwrap();

        let can_send_flight_event = async |event: FlightEvent| {
            let message = FlightEventMessage {
                timestamp: (services.unix_clock.now_ms() as u64).into(),
                event,
            };
            let mut can_tx = can_tx.lock().await;
            can_tx.send(&message, 7).await.ok();
            drop(can_tx);
        };

        loop {
            let state = sub.next_message_pure().await;
            match state {
                FlightCoreStateTelemetry::DisArmed => {}
                FlightCoreStateTelemetry::Armed => {}
                FlightCoreStateTelemetry::PowerAscend => {
                    can_send_flight_event(FlightEvent::Ignition).await;
                }
                FlightCoreStateTelemetry::Coast => {
                    can_send_flight_event(FlightEvent::Coast).await;
                }
                FlightCoreStateTelemetry::Descent => {
                    can_send_flight_event(FlightEvent::Apogee).await;
                }
                FlightCoreStateTelemetry::Landed => {
                    can_send_flight_event(FlightEvent::Landed).await;
                }
            }
        }
    };

    let pyro_main_ctrl_fut = async {
        loop {
            pyro_main_fire_signal.wait().await;
            pyro_main_ctrl.set_enable(true).await.ok();
            services.delay.delay_ms(3000.0).await;
            pyro_main_ctrl.set_enable(false).await.ok();
        }
    };

    let pyro_drogue_ctrl_fut = async {
        loop {
            pyro_drouge_fire_signal.wait().await;
            pyro_drouge_ctrl.set_enable(true).await.ok();
            services.delay.delay_ms(3000.0).await;
            pyro_drouge_ctrl.set_enable(false).await.ok();
        }
    };

    let mut camera_ctrl_ticker = Ticker::every(services.clock(), services.delay(), 1000.0);
    let camera_ctrl_fut = async {
        loop {
            camera_ctrl_ticker.next().await;
            let armed = arming_state.lock(|s| (*s.borrow()).is_armed());
            camera.set_recording(armed).await.ok();
        }
    };

    let mut storage_full_detection_ticker =
        Ticker::every(services.clock(), services.delay(), 1000.0);
    let storage_full_detection_fut = async {
        loop {
            storage_full_detection_ticker.next().await;
            let free = services.fs.free().await;
            storage_full.lock(|s| *s.borrow_mut() = free < 1024 * 1024);
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
        setup_flight_core_fut,
        pyro_main_cont_fut,
        pyro_drogue_cont_fut,
        low_g_imu_fut,
        high_g_imu_fut,
        baro_fut,
        gps_fut,
        mag_fut,
        bat_fut,
        flight_core_tick_fut,
        pyro_main_ctrl_fut,
        pyro_drogue_ctrl_fut,
        flight_core_event_consumer,
        flight_core_state_sub_fut,
        camera_ctrl_fut,
        can_tx_avionics_status_fut,
        can_tx_unix_time_fut,
        indicators_fut,
        storage_full_detection_fut
    );
    log_unreachable!();
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ArmingState {
    pub hardware_armed: bool,
    pub software_armed: bool,
}

impl ArmingState {
    pub fn is_armed(&self) -> bool {
        self.hardware_armed && self.software_armed
    }
}
