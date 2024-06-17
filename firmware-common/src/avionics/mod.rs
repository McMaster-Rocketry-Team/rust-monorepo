use core::cell::RefCell;

use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex},
    channel::{Channel, Sender},
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
    signal::Signal,
};
use flight_profile::FlightProfile;
use futures::join;
use nalgebra::Vector3;
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};
use vlfs::{AsyncWriter, Crc, FileWriter, Flash};

use self::flight_core::Config as FlightCoreConfig;
use crate::{
    allocator::HEAP,
    avionics::up_right_vector_file::write_up_right_vector,
    common::{
        buzzer_queue::BuzzerTone,
        config_file::ConfigFile,
        config_structs::{DeviceConfig, DeviceModeConfig},
        delta_logger::{
            BufferedTieredRingDeltaLogger, TieredRingDeltaLogger, TieredRingDeltaLoggerConfig,
        },
        file_types::*,
        telemetry::telemetry_data::{AvionicsState, TelemetryData},
        vlp2::{
            packet::VLPUplinkPacket, telemetry_packet::TelemetryPacketBuilder,
            uplink_client::VLPUplinkClient,
        },
    },
    driver::{
        imu::IMUReading,
        timestamp::{BootTimestamp, UnixTimestamp},
    },
    vlp::application_layer::{ApplicationLayerRxPackage, ApplicationLayerTxPackage},
};
use crate::{
    avionics::{
        flight_core::{FlightCore, Variances},
        flight_core_event::FlightCoreEvent,
        up_right_vector_file::read_up_right_vector,
    },
    claim_devices,
    common::{
        device_manager::prelude::*,
        file_types::{AVIONICS_LOG_FILE_TYPE, AVIONICS_SENSORS_FILE_TYPE},
        gps_parser::{GPSLocation, GPSParser},
        imu_calibration_file::read_imu_calibration_file,
        sensor_snapshot::{BatteryVoltage, PartialSensorSnapshot, SensorReading},
        ticker::Ticker,
    },
    device_manager_type,
    driver::{
        barometer::BaroReading, debugger::DebuggerTargetEvent, gps::GPS, indicator::Indicator,
        meg::MegReading,
    },
};
use heapless::Vec;
use self_test::{self_test, SelfTestResult};

pub mod avionics_state;
pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;
pub mod flight_profile;
mod self_test;
mod up_right_vector_file;

#[inline(never)]
pub async fn avionics_main(
    device_manager: device_manager_type!(),
    services: system_services_type!(),
    config: &DeviceConfig,
) -> ! {
    let lora_key = if let DeviceModeConfig::Avionics { lora_key } = &config.mode {
        lora_key
    } else {
        log_unreachable!()
    };

    // Init 1KiB heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

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
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(3000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(3000, 50, 150));
        }
        SelfTestResult::PartialFailed => {
            log_warn!("Self test partially failed");
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(3000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
        }
        SelfTestResult::Failed => {
            log_error!("Self test failed");
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(2000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(3000, 50, 150));
            services.buzzer_queue.publish(BuzzerTone(3000, 50, 150));
            indicators.run([200, 200], [], []).await;
        }
    }

    log_info!("Creating loggers");
    let sensor_logger_config = TieredRingDeltaLoggerConfig::new(60, 20 * 60, 10 * 60, 4 * 60 * 60);
    let gps_logger = BufferedTieredRingDeltaLogger::<GPSLocation, _, _, 1>::new(
        services.fs,
        &sensor_logger_config,
        AVIONICS_GPS_LOGGER_TIER_1,
        10,
        AVIONICS_GPS_LOGGER_TIER_2,
        1,
    )
    .await
    .unwrap();
    let gps_logger_fut = gps_logger.run();
    let low_g_imu_logger =
        BufferedTieredRingDeltaLogger::<IMUReading<UnixTimestamp>, _, _, 10>::new(
            services.fs,
            &sensor_logger_config,
            AVIONICS_LOW_G_IMU_LOGGER_TIER_1,
            200,
            AVIONICS_LOW_G_IMU_LOGGER_TIER_2,
            10,
        )
        .await
        .unwrap();
    let low_g_imu_logger_fut = low_g_imu_logger.run();
    let high_g_imu_logger =
        BufferedTieredRingDeltaLogger::<IMUReading<UnixTimestamp>, _, _, 10>::new(
            services.fs,
            &sensor_logger_config,
            AVIONICS_HIGH_G_IMU_LOGGER_TIER_1,
            200,
            AVIONICS_HIGH_G_IMU_LOGGER_TIER_2,
            10,
        )
        .await
        .unwrap();
    let high_g_imu_logger_fut = high_g_imu_logger.run();
    let baro_logger = BufferedTieredRingDeltaLogger::<IMUReading<UnixTimestamp>, _, _, 10>::new(
        services.fs,
        &sensor_logger_config,
        AVIONICS_BARO_LOGGER_TIER_1,
        200,
        AVIONICS_BARO_LOGGER_TIER_2,
        10,
    )
    .await
    .unwrap();
    let baro_logger_fut = baro_logger.run();
    let meg_logger = BufferedTieredRingDeltaLogger::<IMUReading<UnixTimestamp>, _, _, 10>::new(
        services.fs,
        &sensor_logger_config,
        AVIONICS_MEG_LOGGER_TIER_1,
        20,
        AVIONICS_MEG_LOGGER_TIER_2,
        1,
    )
    .await
    .unwrap();
    let meg_logger_fut = meg_logger.run();
    let battery_logger = BufferedTieredRingDeltaLogger::<IMUReading<UnixTimestamp>, _, _, 10>::new(
        services.fs,
        &sensor_logger_config,
        AVIONICS_BATTERY_LOGGER_TIER_1,
        50,
        AVIONICS_BATTERY_LOGGER_TIER_2,
        1,
    )
    .await
    .unwrap();
    let battery_logger_fut = battery_logger.run();
    let total_logger_max_size = gps_logger.max_total_file_size()
        + low_g_imu_logger.max_total_file_size()
        + high_g_imu_logger.max_total_file_size()
        + baro_logger.max_total_file_size()
        + meg_logger.max_total_file_size()
        + battery_logger.max_total_file_size();
    log_info!(
        "Loggers created, total logger max size: {}",
        total_logger_max_size
    );

    let landed = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let sensors_file_should_write_all = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    claim_devices!(
        device_manager,
        arming_switch,
        imu,
        barometer,
        gps,
        meg,
        batt_voltmeter,
        pyro1_cont,
        pyro2_cont,
        lora,
        camera
    );

    let telemetry_packet_builder = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(
        TelemetryPacketBuilder::new(services.unix_clock),
    ));
    let vlp = VLPUplinkClient::new(&config.lora, services.unix_clock, services.delay, lora_key);
    let vlp_tx_fut = async {
        // Wait 1 sec for all the fields to be populated
        services.delay.delay_ms(1000).await;

        let mut update_ticker = Ticker::every(services.clock, services.delay, 100.0);
        loop {
            let packet = telemetry_packet_builder.lock(|b| {
                let mut b = b.borrow();
                b.create_packet()
            });
            vlp.send(packet);
            update_ticker.next().await;
        }
    };
    let vlp_rx_fut = async {
        loop {
            let (packet, status) = vlp.wait_receive().await;
            match packet {
                VLPUplinkPacket::VerticalCalibrationPacket(_) => todo!(),
                VLPUplinkPacket::SoftArmPacket(_) => todo!(),
                VLPUplinkPacket::LowPowerModePacket(_) => todo!(),
                VLPUplinkPacket::ResetPacket(_) => todo!(),
            }
        }
    };
    let vlp_fut = vlp.run(&mut lora);

    let imu = Mutex::<NoopRawMutex, _>::new(imu);

    let cal_info = read_imu_calibration_file(fs).await;

    let flight_core_events = Channel::<NoopRawMutex, FlightCoreEvent, 3>::new();

    let gps_fut = gps_parser.run(&mut gps);

    let arming_state = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(ArmingState {
        hardware_armed: false,
        software_armed: false,
    }));

    let mut delay = device_manager.delay;
    let main_fut = async {
        // TODO buzzer
        let rocket_upright_acc: BlockingMutex<NoopRawMutex, RefCell<Option<Vector3<f32>>>> =
            BlockingMutex::new(RefCell::new(read_up_right_vector(fs).await));
        let flight_core: Mutex<
            NoopRawMutex,
            Option<FlightCore<Sender<NoopRawMutex, FlightCoreEvent, 3>>>,
        > = Mutex::new(None);

        let mut imu_ticker = Ticker::every(clock, delay, 5.0);
        let imu_channel = PubSubChannel::<NoopRawMutex, IMUReading<BootTimestamp>, 1, 1, 1>::new();
        let mut baro_ticker = Ticker::every(clock, delay, 5.0);
        let baro_channel =
            PubSubChannel::<NoopRawMutex, BaroReading<BootTimestamp>, 1, 1, 1>::new();
        let mut meg_ticker = Ticker::every(clock, delay, 50.0);
        let mut batt_volt_ticker = Ticker::every(clock, delay, 5.0);

        let pyro1_cont_fut = async {
            let cont = pyro1_cont.read_continuity().await.unwrap();
            telemetry_data.lock(|d| {
                let mut d = d.borrow_mut();
                d.pyro1_cont = cont
            });

            loop {
                let cont = pyro1_cont.wait_continuity_change().await.unwrap();
                telemetry_data.lock(|d| {
                    let mut d = d.borrow_mut();
                    d.pyro1_cont = cont
                });
            }
        };

        let pyro2_cont_fut = async {
            let cont = pyro2_cont.read_continuity().await.unwrap();
            telemetry_data.lock(|d| {
                let mut d = d.borrow_mut();
                d.pyro2_cont = cont
            });

            loop {
                let cont = pyro2_cont.wait_continuity_change().await.unwrap();
                telemetry_data.lock(|d| {
                    let mut d = d.borrow_mut();
                    d.pyro2_cont = cont
                });
            }
        };

        let imu_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed())
                    && let Ok(mut imu) = imu.try_lock()
                {
                    let imu_reading = imu.read().await.unwrap();
                    sensors_file_channel.publish_immediate(SensorReading::IMU(imu_reading.clone()));
                    imu_channel.publish_immediate(imu_reading);
                }
                imu_ticker.next().await;
            }
        };

        let baro_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed()) {
                    let baro_reading = barometer.read().await.unwrap();
                    telemetry_data.lock(|d| {
                        let mut d = d.borrow_mut();
                        d.pressure = baro_reading.pressure;
                        d.temperature = baro_reading.temperature;
                    });
                    sensors_file_channel
                        .publish_immediate(SensorReading::Baro(baro_reading.clone()));
                    baro_channel.publish_immediate(baro_reading);
                }
                baro_ticker.next().await;
            }
        };

        let gps_logging_fut = async {
            loop {
                if gps_parser.get_updated() {
                    let nmea = gps_parser.get_nmea();
                    telemetry_data.lock(|d| {
                        let mut d = d.borrow_mut();
                        d.satellites_in_use = nmea.num_of_fix_satellites as u32;
                        d.lat_lon = nmea.lat_lon;
                    });
                    sensors_file_channel.publish_immediate(SensorReading::GPS(nmea));
                }

                delay.delay_ms(5).await;
            }
        };

        let meg_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed()) {
                    if let Ok(meg_reading) = meg.read().await {
                        sensors_file_channel.publish_immediate(SensorReading::Meg(meg_reading));
                    } else {
                        log_warn!("Failed to read meg")
                    }
                }
                meg_ticker.next().await;
            }
        };

        let bat_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed()) {
                    let batt_volt_reading = batt_voltmeter.read().await.unwrap();
                    telemetry_data.lock(|d| {
                        let mut d = d.borrow_mut();
                        d.battery_voltage = batt_volt_reading;
                    });
                    sensors_file_channel.publish_immediate(SensorReading::BatteryVoltage(
                        BatteryVoltage {
                            timestamp: clock.now_ms(),
                            voltage: batt_volt_reading,
                        },
                    ));
                }
                batt_volt_ticker.next().await;
            }
        };

        let arming_fut = async {
            let mut last_arming_state = arming_state.lock(|s| (*s.borrow()).is_armed());
            loop {
                let new_arming_state = arming_switch.wait_arming_change().await.unwrap();
                telemetry_data.lock(|s| {
                    s.borrow_mut().hardware_armed = new_arming_state;
                });
                let new_arming_state = arming_state.lock(|s| {
                    let mut s = s.borrow_mut();
                    s.hardware_armed = new_arming_state;
                    s.is_armed()
                });
                if new_arming_state != last_arming_state {
                    if new_arming_state {
                        if let Some(rocket_upright_acc) = rocket_upright_acc.lock(|s| *s.borrow()) {
                            let variances = if let Some(cal_info) = &cal_info {
                                Variances::from_imu_cal_info(cal_info, 2.0)
                            } else {
                                Variances::default()
                            };
                            let mut flight_core = flight_core.lock().await;
                            flight_core.replace(FlightCore::new(
                                MARAUDER_2_FLIGHT_CONFIG,
                                flight_core_events.sender(),
                                rocket_upright_acc,
                                variances,
                            ));
                            drop(flight_core);

                            let mut tones = Vec::new();
                            tones.push(BuzzerTone(Some(2000), 500)).unwrap();
                            tones.push(BuzzerTone(None, 500)).unwrap();
                            tones.push(BuzzerTone(Some(3000), 500)).unwrap();
                            shared_buzzer_channel.publish_immediate(tones);
                        } else {
                            let mut tones = Vec::new();
                            tones.push(BuzzerTone(Some(3000), 500)).unwrap();
                            tones.push(BuzzerTone(None, 500)).unwrap();
                            tones.push(BuzzerTone(Some(2000), 500)).unwrap();
                            shared_buzzer_channel.publish_immediate(tones);
                        }
                    } else {
                        telemetry_data.lock(|s| {
                            s.borrow_mut().avionics_state = AvionicsState::Idle;
                        });
                        let mut flight_core = flight_core.lock().await;
                        flight_core.take();
                    }
                }
                last_arming_state = new_arming_state;
            }
        };

        let mut delay = device_manager.delay;
        let radio_fut = async {
            loop {
                match radio_rx.receive().await {
                    ApplicationLayerRxPackage::VerticalCalibration => {
                        log_info!("Vertical calibration");
                        if !arming_state.lock(|s| (*s.borrow()).is_armed()) {
                            let mut ticker = Ticker::every(clock, delay, 1.0);
                            let mut acc_sum = Vector3::<f32>::zeros();
                            let mut imu = imu.lock().await;
                            for _ in 0..100 {
                                let mut reading = imu.read().await.unwrap();
                                if let Some(cal_info) = &cal_info {
                                    reading = cal_info.apply_calibration(reading);
                                }
                                acc_sum += Vector3::from(reading.acc);
                                ticker.next().await;
                            }
                            drop(imu);
                            acc_sum /= 100.0;
                            rocket_upright_acc.lock(|s| s.borrow_mut().replace(acc_sum));
                            write_up_right_vector(fs, acc_sum).await.unwrap();

                            let mut tones = Vec::new();
                            tones.push(BuzzerTone(Some(2700), 500)).unwrap();
                            tones.push(BuzzerTone(None, 250)).unwrap();
                            tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                            tones.push(BuzzerTone(None, 150)).unwrap();
                            tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                            shared_buzzer_channel.publish_immediate(tones);
                        }
                    }
                    ApplicationLayerRxPackage::ClearStorage => {
                        log_info!("Clearing storage");
                        fs.remove_files(|file| {
                            file.typ == AVIONICS_LOG_FILE_TYPE
                                || file.typ == AVIONICS_SENSORS_FILE_TYPE
                        })
                        .await
                        .unwrap();
                        let mut tones = Vec::new();
                        tones.push(BuzzerTone(Some(2700), 500)).unwrap();
                        tones.push(BuzzerTone(None, 250)).unwrap();
                        tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                        tones.push(BuzzerTone(None, 150)).unwrap();
                        tones.push(BuzzerTone(Some(3700), 50)).unwrap();
                        shared_buzzer_channel.publish_immediate(tones);
                    }
                    ApplicationLayerRxPackage::SoftArming(software_armed) => {
                        arming_state.lock(|s| s.borrow_mut().software_armed = software_armed);
                        telemetry_data.lock(|s| {
                            s.borrow_mut().software_armed = software_armed;
                        });
                    }
                }
            }
        };

        let mut delay = device_manager.delay;
        let flight_core_fut = async {
            let mut imu_sub = imu_channel.subscriber().unwrap();
            let mut baro_sub = baro_channel.subscriber().unwrap();
            loop {
                // would love to use chained if lets, but rustfmt doesn't like it
                if arming_state.lock(|s| (*s.borrow()).is_armed()) {
                    if let Ok(mut flight_core) = flight_core.try_lock() {
                        if let Some(flight_core) = flight_core.as_mut() {
                            let imu_reading = imu_sub.next_message_pure().await;
                            let baro_reading = baro_sub.try_next_message_pure();

                            let sensor_snapshot = PartialSensorSnapshot {
                                timestamp: imu_reading.timestamp,
                                imu_reading,
                                baro_reading,
                            };

                            flight_core.tick(sensor_snapshot);
                            continue;
                        }
                    }
                }
                delay.delay_ms(5).await;
            }
        };

        #[allow(unreachable_code)]
        {
            join!(
                imu_fut,
                baro_fut,
                arming_fut,
                radio_fut,
                flight_core_fut,
                gps_fut,
                gps_logging_fut,
                meg_fut,
                bat_fut,
                pyro1_cont_fut,
                pyro2_cont_fut
            );
        }
    };

    let mut delay = device_manager.delay;
    let telemetry_fut = async {
        let mut telemetry_ticker = Ticker::every(clock, delay, 3000.);
        let mut last_telemetry_timestamp = -60_000.0f64;
        loop {
            let should_throttle = !arming_state.lock(|s| (*s.borrow()).is_armed())
                || telemetry_data.lock(|d| d.borrow().avionics_state == AvionicsState::Landed);
            if should_throttle && clock.now_ms() - last_telemetry_timestamp <= 60_000.0 {
                telemetry_ticker.next().await;
                continue;
            }

            let mut telemetry_data = telemetry_data.lock(|d| d.borrow().clone());
            telemetry_data.timestamp = clock.now_ms();
            last_telemetry_timestamp = telemetry_data.timestamp;
            log_info!("Sending telemetry {:?}", telemetry_data);
            radio_tx
                .send(ApplicationLayerTxPackage::Telemetry(telemetry_data))
                .await;
            telemetry_ticker.next().await;
        }
    };

    let mut delay = device_manager.delay;
    let camera_ctrl_future = async {
        let mut ticker = Ticker::every(clock, delay, 3000.0);
        let mut is_recording = false;

        loop {
            let should_record = arming_state.lock(|s| (*s.borrow()).is_armed())
                && telemetry_data.lock(|d| d.borrow().avionics_state != AvionicsState::Landed);
            if should_record && !is_recording {
                camera.set_recording(true).await;
                is_recording = true;
            } else if !should_record && is_recording {
                delay.delay_ms(60_000).await;
                camera.set_recording(false).await;
                is_recording = false;
            }
            ticker.next().await;
        }
    };

    let mut delay = device_manager.delay;
    let flight_core_event_consumer = async {
        let receiver = flight_core_events.receiver();
        // pyro1: main, pyro2: drogue
        claim_devices!(device_manager, pyro1_ctrl, pyro2_ctrl);
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
                    // TODO cameras
                    sensors_file_should_write_all.lock(|s| *s.borrow_mut() = true);
                }
                FlightCoreEvent::Apogee => {
                    // noop
                }
                FlightCoreEvent::DeployMain => {
                    pyro1_ctrl.set_enable(true).await.ok();
                    delay.delay_ms(3000).await;
                    pyro1_ctrl.set_enable(false).await.ok();
                }
                FlightCoreEvent::DeployDrogue => {
                    pyro2_ctrl.set_enable(true).await.ok();
                    delay.delay_ms(3000).await;
                    pyro2_ctrl.set_enable(false).await.ok();
                }
                FlightCoreEvent::Landed => {
                    landed.lock(|s| *s.borrow_mut() = true);
                    sensors_file_should_write_all.lock(|s| *s.borrow_mut() = false);
                }
                FlightCoreEvent::DidNotReachMinApogee => {
                    // noop
                }
                FlightCoreEvent::ChangeState(new_state) => {
                    telemetry_data.lock(|s| s.borrow_mut().avionics_state = new_state);
                }
                FlightCoreEvent::ChangeAltitude(new_altitude) => {
                    telemetry_data.lock(|s| {
                        let mut s = s.borrow_mut();
                        s.altitude = new_altitude;
                        if new_altitude > s.max_altitude {
                            s.max_altitude = new_altitude;
                        }
                    });
                }
            }
        }
    };

    let delay = device_manager.delay;
    let mut landed_buzzing_ticker = Ticker::every(clock, delay, 5000.0);
    let landed_buzzing_fut = async {
        loop {
            if landed.lock(|s| *s.borrow()) {
                let mut tones = Vec::new();
                tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                tones.push(BuzzerTone(None, 150)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                tones.push(BuzzerTone(None, 500)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                tones.push(BuzzerTone(None, 150)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50)).unwrap();
                shared_buzzer_channel.publish_immediate(tones);
            }
            landed_buzzing_ticker.next().await;
        }
    };

    join!(
        landed_buzzing_fut,
        shared_buzzer_fut,
        telemetry_fut,
        radio_fut,
        main_fut,
        flight_core_event_consumer,
        camera_ctrl_future,
    );
    log_unreachable!();
}

const MARAUDER_2_FLIGHT_CONFIG: FlightCoreConfig = FlightCoreConfig {
    drogue_chute_minimum_time_ms: 20_000.0,
    drogue_chute_minimum_altitude_agl: 2000.0,
    drogue_chute_delay_ms: 2000.0,
    main_chute_delay_ms: 0.0,
    main_chute_altitude_agl: 365.0, // 1200 ft
};

struct ArmingState {
    pub hardware_armed: bool,
    pub software_armed: bool,
}

impl ArmingState {
    pub fn is_armed(&self) -> bool {
        self.hardware_armed && self.software_armed
    }
}
