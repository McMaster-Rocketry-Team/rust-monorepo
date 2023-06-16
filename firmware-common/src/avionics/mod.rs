use core::cell::RefCell;

use defmt::unwrap;
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    blocking_mutex::Mutex as BlockingMutex,
    channel::{Channel, Sender},
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
};
use ferraris_calibration::IMUReading;
use futures::join;
use lora_phy::mod_traits::RadioKind;
use nalgebra::Vector3;
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};
use vlfs::{io_traits::AsyncWriter, Crc, FileWriter, Flash, VLFS};

use crate::{
    avionics::{
        flight_core::{FlightCore, Variances},
        flight_core_event::FlightCoreEvent,
    },
    claim_devices,
    common::{
        device_manager::prelude::*,
        files::{AVIONICS_LOG_FILE_TYPE, AVIONICS_SENSORS_FILE_TYPE, CALIBRATION_FILE_TYPE},
        gps_parser::GPSParser,
        imu_calibration_file::read_imu_calibration_file,
        sensor_snapshot::{PartialSensorSnapshot, SensorReading},
        ticker::Ticker,
    },
    device_manager_type,
    driver::{
        barometer::BaroReading,
        debugger::{
            ApplicationLayerRxPackage, ApplicationLayerTxPackage, DebuggerTargetEvent,
            RadioApplicationClient,
        },
        gps::GPS,
        indicator::Indicator,
        timer::Timer,
    },
};

use self::flight_core::Config as FlightCoreConfig;

pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;

async fn save_sensor_reading(
    reading: SensorReading,
    sensors_file: &mut FileWriter<'_, impl Flash, impl Crc>,
    buffer: [u8; 100],
) -> [u8; 100] {
    let mut serializer = BufferSerializer::new(buffer);
    serializer.serialize_value(&reading).unwrap();
    let buffer = serializer.into_inner();
    let buffer_slice = &buffer[..core::mem::size_of::<<SensorReading as Archive>::Archived>()];
    sensors_file.extend_from_slice(buffer_slice).await.unwrap();

    buffer
}

#[inline(never)]
pub async fn avionics_main(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) -> ! {
    let timer = device_manager.timer;
    claim_devices!(device_manager, buzzer);

    let sensors_file_id = unwrap!(fs.create_file(AVIONICS_SENSORS_FILE_TYPE).await.ok());
    let mut sensors_file = unwrap!(fs.open_file_for_write(sensors_file_id).await.ok());
    let log_file_id = unwrap!(fs.create_file(AVIONICS_LOG_FILE_TYPE).await.ok());
    let mut logs_file = unwrap!(fs.open_file_for_write(log_file_id).await.ok());

    let sensors_file_should_write_all = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let sensors_file_channel = PubSubChannel::<NoopRawMutex, SensorReading, 200, 1, 1>::new();

    let sensors_file_fut = async {
        let write_interval_ms = 1000.0;
        let mut last_gps_timestamp = 0.0f64;
        let mut last_imu_timestamp = 0.0f64;
        let mut last_baro_timestamp = 0.0f64;
        let mut last_meg_timestamp = 0.0f64;
        let mut last_batt_volt = 0.0f64;
        let mut buffer = [0u8; 100];
        let mut subscriber = sensors_file_channel.subscriber().unwrap();

        loop {
            let sensor_reading = subscriber.next_message_pure().await;
            if sensors_file_should_write_all.lock(|v| *v.borrow()) {
                buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer).await;
            } else {
                match &sensor_reading {
                    SensorReading::GPS(gps_reading) => {
                        if gps_reading.timestamp - last_gps_timestamp > write_interval_ms {
                            last_gps_timestamp = gps_reading.timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                    SensorReading::IMU(imu_reading) => {
                        if imu_reading.timestamp - last_imu_timestamp > write_interval_ms {
                            last_imu_timestamp = imu_reading.timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                    SensorReading::Baro(baro_reading) => {
                        if baro_reading.timestamp - last_baro_timestamp > write_interval_ms {
                            last_baro_timestamp = baro_reading.timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                    SensorReading::Meg(meg_reading) => {
                        if meg_reading.timestamp - last_meg_timestamp > write_interval_ms {
                            last_meg_timestamp = meg_reading.timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                    SensorReading::BatteryVoltage { timestamp, .. } => {
                        if timestamp - last_batt_volt > write_interval_ms {
                            last_batt_volt = *timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                }
            }
        }
    };

    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;

    let buzzer = Mutex::<NoopRawMutex, _>::new(buzzer);

    claim_devices!(
        device_manager,
        arming_switch,
        imu,
        barometer,
        gps,
        meg,
        batt_voltmeter
    );
    unwrap!(imu.wait_for_power_on().await);
    unwrap!(imu.reset().await);
    unwrap!(barometer.reset().await);
    unwrap!(meg.reset().await);

    let imu = Mutex::<NoopRawMutex, _>::new(imu);

    let gps_parser = GPSParser::new(timer);

    let cal_info = read_imu_calibration_file(fs).await;

    fs.find_file_by_type(CALIBRATION_FILE_TYPE).await;

    let radio = device_manager.get_radio_application_layer().await;

    let radio_tx = Channel::<CriticalSectionRawMutex, ApplicationLayerTxPackage, 3>::new();
    let radio_rx = Channel::<CriticalSectionRawMutex, ApplicationLayerRxPackage, 3>::new();

    let flight_core_events = Channel::<CriticalSectionRawMutex, FlightCoreEvent, 3>::new();

    let gps_fut = gps_parser.run(&mut gps);

    let radio_fut = async {
        if let Some(mut radio) = radio {
            radio.run(radio_tx.receiver(), radio_rx.sender()).await;
        }
    };

    let main_fut = async {
        let arming_state = unwrap!(arming_switch.read_arming().await);
        let arming_state =
            BlockingMutex::<CriticalSectionRawMutex, _>::new(RefCell::new(arming_state));
        let rocket_upright_acc: BlockingMutex<NoopRawMutex, RefCell<Option<Vector3<f32>>>> =
            BlockingMutex::new(RefCell::new(None));
        let flight_core: Mutex<
            NoopRawMutex,
            Option<FlightCore<Sender<CriticalSectionRawMutex, FlightCoreEvent, 3>>>,
        > = Mutex::new(None);
        let mut imu_ticker = Ticker::every(timer, 5.0);
        let imu_channel = PubSubChannel::<NoopRawMutex, IMUReading, 1, 1, 1>::new();
        let mut baro_ticker = Ticker::every(timer, 5.0);
        let baro_channel = PubSubChannel::<NoopRawMutex, BaroReading, 1, 1, 1>::new();
        let mut meg_ticker = Ticker::every(timer, 50.0);
        let mut batt_volt_ticker = Ticker::every(timer, 5.0);

        let imu_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) && let Ok(mut imu) = imu.try_lock() {
                    let imu_reading = unwrap!(imu.read().await);
                    sensors_file_channel.publish_immediate(SensorReading::IMU(imu_reading.clone()));
                    imu_channel.publish_immediate(imu_reading);
                }
                imu_ticker.next().await;
            }
        };

        let baro_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) {
                    let baro_reading = unwrap!(barometer.read().await);
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
                    sensors_file_channel
                        .publish_immediate(SensorReading::GPS(gps_parser.get_nmea()));
                }

                timer.sleep(5.0).await;
            }
        };

        let meg_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) {
                    let meg_reading = unwrap!(meg.read().await);
                    sensors_file_channel.publish_immediate(SensorReading::Meg(meg_reading));
                }
                meg_ticker.next().await;
            }
        };

        let bat_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) {
                    let batt_volt_reading = unwrap!(batt_voltmeter.read().await);
                    sensors_file_channel.publish_immediate(SensorReading::BatteryVoltage {
                        timestamp: timer.now_mills(),
                        voltage: batt_volt_reading,
                    });
                }
                batt_volt_ticker.next().await;
            }
        };

        let arming_fut = async {
            let mut last_arming_state = arming_state.lock(|s| *s.borrow());
            loop {
                let new_arming_state = unwrap!(arming_switch.wait_arming_change().await);
                arming_state.lock(|s| *s.borrow_mut() = new_arming_state);
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

                            let mut buzzer = buzzer.lock().await;
                            buzzer.play(2000, 500.0).await;
                            timer.sleep(500.0).await;
                            buzzer.play(3000, 500.0).await;
                        } else {
                            let mut buzzer = buzzer.lock().await;
                            buzzer.play(3000, 500.0).await;
                            timer.sleep(500.0).await;
                            buzzer.play(2000, 500.0).await;
                        }
                    } else {
                        let mut flight_core = flight_core.lock().await;
                        flight_core.take();
                    }
                }
                last_arming_state = new_arming_state;
            }
        };

        let radio_fut = async {
            loop {
                match radio_rx.recv().await {
                    ApplicationLayerRxPackage::VerticalCalibration => {
                        log_info!("Vertical calibration");
                        if !arming_state.lock(|s| *s.borrow()) {
                            let mut ticker = Ticker::every(timer, 1.0);
                            let mut acc_sum = Vector3::<f32>::zeros();
                            let mut imu = imu.lock().await;
                            for _ in 0..100 {
                                let mut reading = unwrap!(imu.read().await);
                                if let Some(cal_info) = &cal_info {
                                    reading = cal_info.apply_calibration(&reading);
                                }
                                acc_sum += Vector3::from(reading.acc);
                                ticker.next().await;
                            }
                            drop(imu);
                            rocket_upright_acc.lock(|s| s.borrow_mut().replace(acc_sum / 100.0));

                            let mut buzzer = buzzer.lock().await;
                            buzzer.play(2700, 500.0).await;
                            timer.sleep(250.0).await;
                            buzzer.play(2700, 50.0).await;
                            timer.sleep(150.0).await;
                            buzzer.play(2700, 50.0).await;
                        }
                    }
                    ApplicationLayerRxPackage::SoftArming(_) => {
                        // TODO
                    }
                }
            }
        };

        let flight_core_fut = async {
            let mut imu_sub = imu_channel.subscriber().unwrap();
            let mut baro_sub = baro_channel.subscriber().unwrap();
            loop {
                // would love to use chained if lets, but rustfmt doesn't like it
                if arming_state.lock(|s| *s.borrow()) {
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
                timer.sleep(5.0).await;
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
            );
        }
    };

    let flight_core_event_consumer = async {
        let receiver = flight_core_events.receiver();
        // TODO check again: pyro1: main, pyro2: drogue
        claim_devices!(device_manager, pyro1_ctrl, pyro2_ctrl);
        let debugger = device_manager.debugger.clone();
        loop {
            let event = receiver.recv().await;
            debugger.dispatch(DebuggerTargetEvent::FlightCoreEvent(event));
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
                    timer.sleep(3000.0);
                    pyro1_ctrl.set_enable(false).await.ok();
                }
                FlightCoreEvent::DeployDrogue => {
                    pyro2_ctrl.set_enable(true).await.ok();
                    timer.sleep(3000.0);
                    pyro2_ctrl.set_enable(false).await.ok();
                }
                FlightCoreEvent::Landed => {
                    // TODO buzzer
                    sensors_file_should_write_all.lock(|s| *s.borrow_mut() = false);
                }
                FlightCoreEvent::DidNotReachMinApogee => {
                    // noop
                }
            }
        }
    };

    join!(
        radio_fut,
        main_fut,
        flight_core_event_consumer,
        sensors_file_fut
    );
    defmt::unreachable!();
}

const MARAUDER_2_FLIGHT_CONFIG: FlightCoreConfig = FlightCoreConfig {
    drogue_chute_minimum_time_ms: 20_000.0,
    drogue_chute_minimum_altitude_agl: 2000.0,
    drogue_chute_delay_ms: 1000.0,
    main_chute_delay_ms: 1000.0,
    main_chute_altitude_agl: 457.0, // 1500 ft
};
