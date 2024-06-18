use core::cell::RefCell;

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
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive, Deserialize, Serialize,
};
use vlfs::{AsyncWriter, Crc, FileWriter, Flash};

use crate::{
    allocator::HEAP,
    avionics::up_right_vector_file::write_up_right_vector,
    common::{
        buzzer_queue::{self, BuzzerTone},
        config_file::ConfigFile,
        config_structs::{DeviceConfig, DeviceModeConfig},
        delta_logger::{
            BufferedTieredRingDeltaLogger, TieredRingDeltaLogger, TieredRingDeltaLoggerConfig,
        },
        file_types::*,
        imu_calibration_file,
        telemetry::telemetry_data::{AvionicsState, TelemetryData},
        vlp2::{
            packet::{SoftArmPacket, VLPUplinkPacket},
            packet_builder,
            telemetry_packet::TelemetryPacketBuilder,
            uplink_client::VLPUplinkClient,
        },
    },
    driver::{
        adc::ADCReading, imu::IMUReading, timestamp::{BootTimestamp, UnixTimestamp}
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
mod imu_calibration_info;
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

    claim_devices!(
        device_manager,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        pyro3_cont,
        pyro3_ctrl
    );

    let (mut pyro_main_cont, mut pyro_main_ctrl) = match flight_profile.main_pyro {
        PyroSelection::Pyro1 => (pyro1_cont, pyro1_ctrl),
        PyroSelection::Pyro2 => (pyro2_cont, pyro2_ctrl),
        PyroSelection::Pyro3 => (pyro3_cont, pyro3_ctrl),
    };
    let (mut pyro_drogue_cont, mut pyro_drouge_ctrl) = match flight_profile.drogue_pyro {
        PyroSelection::Pyro1 => (pyro1_cont, pyro1_ctrl),
        PyroSelection::Pyro2 => (pyro2_cont, pyro2_ctrl),
        PyroSelection::Pyro3 => (pyro3_cont, pyro3_ctrl),
    };

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
    let baro_logger = BufferedTieredRingDeltaLogger::<BaroReading<UnixTimestamp>, _, _, 10>::new(
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
    let meg_logger = BufferedTieredRingDeltaLogger::<MegReading<UnixTimestamp>, _, _, 10>::new(
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
    let battery_logger = BufferedTieredRingDeltaLogger::<ADCReading<Volt,UnixTimestamp>, _, _, 10>::new(
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

    // states
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

    let low_g_imu_signal = Signal::<NoopRawMutex, IMUReading<BootTimestamp>>::new();
    let high_g_imu_signal = Signal::<NoopRawMutex, IMUReading<BootTimestamp>>::new();
    let baro_signal = Signal::<NoopRawMutex, BaroReading<BootTimestamp>>::new();

    let imu_config_file = ConfigFile::<IMUCalibrationInfo, _, _>::new(
        services.fs,
        UPRIGHT_VECTOR_AND_GYRO_OFFSET_FILE_TYPE,
    );
    let mut imu_config =
        BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(imu_config_file.read().await));

    claim_devices!(
        device_manager,
        arming_switch,
        imu,
        barometer,
        gps,
        meg,
        batt_voltmeter,
        lora,
        camera,
        sys_reset
    );

    let telemetry_packet_builder = TelemetryPacketBuilder::new(services.unix_clock);
    let vlp = VLPUplinkClient::new(&config.lora, services.unix_clock, services.delay, lora_key);
    let vlp_tx_fut = async {
        // Wait 1 sec for all the fields to be populated
        services.delay.delay_ms(1000).await;

        let mut update_ticker = Ticker::every(services.clock, services.delay, 100.0);
        loop {
            let packet = telemetry_packet_builder.create_packet();
            vlp.send(packet);
            update_ticker.next().await;
        }
    };
    let vlp_rx_fut = async {
        loop {
            let (packet, status) = vlp.wait_receive().await;
            match packet {
                VLPUplinkPacket::VerticalCalibrationPacket(_) => {
                    log_info!("Vertical calibration");
                    if services.unix_clock.ready()
                        && !arming_state.lock(|s| (*s.borrow()).is_armed())
                    {
                        let mut acc_sum = Vector3::<f32>::zeros();
                        let mut gyro_sum = Vector3::<f32>::zeros();
                        for _ in 0..100 {
                            let mut reading = low_g_imu_signal.wait().await;
                            acc_sum += Vector3::from(reading.acc);
                            gyro_sum += Vector3::from(reading.gyro);
                        }
                        acc_sum /= 100.0;
                        gyro_sum /= 100.0;

                        let new_imu_config = IMUCalibrationInfo {
                            gyro_offset: (-gyro_sum).into(),
                            up_right_vector: acc_sum.into(),
                        };
                        imu_config_file.write(&new_imu_config);
                        imu_config.lock(|s| s.borrow_mut().replace(new_imu_config));
                    }
                }
                VLPUplinkPacket::SoftArmPacket(SoftArmPacket { armed, .. }) => {
                    arming_state.lock(|s| s.borrow_mut().software_armed = armed);
                    arming_changed_signal.signal(());
                    telemetry_packet_builder.update(|b| {
                        b.software_armed = armed;
                    });
                }
                VLPUplinkPacket::LowPowerModePacket(_) => todo!(),
                VLPUplinkPacket::ResetPacket(_) => {
                    sys_reset.reset();
                }
            }
        }
    };
    let vlp_fut = vlp.run(&mut lora);

    let hardware_arming_fut = async {
        let mut hardware_armed = arming_switch.read_arming().await.unwrap();
        if hardware_armed {
            services.buzzer_queue.publish(BuzzerTone(2000, 700, 300));
            services.buzzer_queue.publish(BuzzerTone(3000, 700, 300));
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
                services.buzzer_queue.publish(BuzzerTone(2000, 700, 300));
                services.buzzer_queue.publish(BuzzerTone(3000, 700, 300));
            } else {
                services.buzzer_queue.publish(BuzzerTone(3000, 700, 300));
                services.buzzer_queue.publish(BuzzerTone(2000, 700, 300));
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
                            flight_profile,
                            flight_core_events.sender(),
                            imu_config.up_right_vector.into(),
                            Variances::default(),
                        ));
                    })
                }
            } else if !armed && flight_core_initialized {
                flight_core.lock(|s| s.take());
            }
        }
    };

    let flight_core_events_receiver_fut = async {
        let receiver = flight_core_events.receiver();
        loop {
            let event = receiver.receive().await;
            match event {
                FlightCoreEvent::CriticalError => todo!(),
                FlightCoreEvent::Ignition => todo!(),
                FlightCoreEvent::Apogee => todo!(),
                FlightCoreEvent::DeployMain => todo!(),
                FlightCoreEvent::DeployDrogue => todo!(),
                FlightCoreEvent::Landed => todo!(),
                FlightCoreEvent::DidNotReachMinApogee => todo!(),
                FlightCoreEvent::ChangeState(_) => todo!(),
                FlightCoreEvent::ChangeAltitude(_) => todo!(),
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

    let mut low_g_imu_ticker = Ticker::every(services.clock, services.delay, 5.0);
    let low_g_imu_fut = async {
        loop {
            let imu_reading = imu.read().await.unwrap();
            low_g_imu_signal.signal(imu_reading.clone());
            if services.unix_clock.ready() {
                low_g_imu_logger
                    .log(imu_reading.to_unix_timestamp(services.unix_clock))
                    .await;
            }
            low_g_imu_ticker.next().await;
        }
    };

    let mut high_g_imu_ticker = Ticker::every(services.clock, services.delay, 5.0);
    let high_g_imu_fut = async {
        loop {
            let imu_reading = imu.read().await.unwrap();
            high_g_imu_signal.signal(imu_reading.clone());
            if services.unix_clock.ready() {
                high_g_imu_logger
                    .log(imu_reading.to_unix_timestamp(services.unix_clock))
                    .await;
            }
            high_g_imu_ticker.next().await;
        }
    };

    let mut baro_ticker = Ticker::every(services.clock, services.delay, 5.0);
    let baro_fut = async {
        loop {
            let baro_reading = barometer.read().await.unwrap();
            baro_signal.signal(baro_reading.clone());
            if services.unix_clock.ready() {
                baro_logger
                    .log(baro_reading.to_unix_timestamp(services.unix_clock))
                    .await;
            }
            baro_ticker.next().await;
        }
    };

    let mut gps_ticker = Ticker::every(services.clock, services.delay, 100.0);
    let gps_fut = async {
        loop {
            let gps_location = services.gps.get_nmea();
            gps_logger.log(gps_location).await;
            gps_ticker.next().await;
        }
    };

    let mut meg_ticker = Ticker::every(services.clock, services.delay, 50.0);
    let meg_fut = async {
        loop {
            let meg_reading = meg.read().await.unwrap();
            if services.unix_clock.ready() {
                meg_logger
                    .log(meg_reading.to_unix_timestamp(services.unix_clock))
                    .await;
            }
            meg_ticker.next().await;
        }
    };

    let mut batt_volt_ticker = Ticker::every(services.clock, services.delay, 5.0);
    let bat_fut = async {
        loop {
            let battery_v = batt_voltmeter.read().await.unwrap();
            if services.unix_clock.ready() {
                battery_logger
                    .log(battery_v.to_unix_timestamp(services.unix_clock))
                    .await;
            }
            telemetry_packet_builder.update(|b| {
                b.battery_v = battery_v.value;
            });
            batt_volt_ticker.next().await;
        }
    };

    let flight_core_fut = async {

    };

    let main_fut = async {
        let delay = services.delay;
        let clock = services.clock;
        let flight_core: Mutex<
            NoopRawMutex,
            Option<FlightCore<Sender<NoopRawMutex, FlightCoreEvent, 3>>>,
        > = Mutex::new(None);


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
