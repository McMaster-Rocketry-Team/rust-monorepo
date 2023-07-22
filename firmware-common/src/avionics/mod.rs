use core::cell::RefCell;

use defmt::unwrap;
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    blocking_mutex::Mutex as BlockingMutex,
    channel::{Channel, Sender},
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
};
use ferraris_calibration::IMUReading;
use futures::join;
use nalgebra::Vector3;
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};
use vlfs::{io_traits::AsyncWriter, Crc, FileWriter, Flash, VLFS};

use self::flight_core::Config as FlightCoreConfig;
use crate::{
    allocator::HEAP,
    avionics::up_right_vector_file::write_up_right_vector,
    common::{
        pvlp::{PVLPMaster, PVLP},
        telemetry::telemetry_data::{AvionicsState, TelemetryData},
        vlp::{SocketParams, VLPSocket},
    },
    vlp::{
        application_layer::{
            ApplicationLayerRxPackage, ApplicationLayerTxPackage, RadioApplicationPackage,
        },
        Priority,
    },
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
        files::{AVIONICS_LOG_FILE_TYPE, AVIONICS_SENSORS_FILE_TYPE},
        gps_parser::{GPSLocation, GPSParser},
        imu_calibration_file::read_imu_calibration_file,
        sensor_snapshot::{BatteryVoltage, PartialSensorSnapshot, SensorReading},
        ticker::Ticker,
    },
    device_manager_type,
    driver::{
        barometer::BaroReading, debugger::DebuggerTargetEvent, gps::GPS, indicator::Indicator,
        meg::MegReading, timer::Timer,
    },
};
use heapless::Vec;
use self_test::self_test;

pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;
mod self_test;
mod up_right_vector_file;

async fn save_sensor_reading(
    reading: SensorReading,
    sensors_file: &mut FileWriter<'_, impl Flash, impl Crc>,
    buffer: [u8; 100],
) -> [u8; 100] {
    let mut serializer = BufferSerializer::new(buffer);
    match reading {
        SensorReading::GPS(gps) => {
            serializer.serialize_value(&gps).unwrap();
            let buffer = serializer.into_inner();
            let buffer_slice =
                &buffer[..core::mem::size_of::<<GPSLocation as Archive>::Archived>()];
            sensors_file.extend_from_u8(0).await.unwrap();
            sensors_file.extend_from_slice(buffer_slice).await.unwrap();

            buffer
        }
        SensorReading::IMU(imu) => {
            serializer.serialize_value(&imu).unwrap();
            let buffer = serializer.into_inner();
            let buffer_slice = &buffer[..core::mem::size_of::<<IMUReading as Archive>::Archived>()];
            sensors_file.extend_from_u8(1).await.unwrap();
            sensors_file.extend_from_slice(buffer_slice).await.unwrap();

            buffer
        }
        SensorReading::Baro(baro) => {
            serializer.serialize_value(&baro).unwrap();
            let buffer = serializer.into_inner();
            let buffer_slice =
                &buffer[..core::mem::size_of::<<BaroReading as Archive>::Archived>()];
            sensors_file.extend_from_u8(2).await.unwrap();
            sensors_file.extend_from_slice(buffer_slice).await.unwrap();

            buffer
        }
        SensorReading::Meg(meg) => {
            serializer.serialize_value(&meg).unwrap();
            let buffer = serializer.into_inner();
            let buffer_slice = &buffer[..core::mem::size_of::<<MegReading as Archive>::Archived>()];
            sensors_file.extend_from_u8(3).await.unwrap();
            sensors_file.extend_from_slice(buffer_slice).await.unwrap();

            buffer
        }
        SensorReading::BatteryVoltage(battery_voltage) => {
            serializer.serialize_value(&battery_voltage).unwrap();
            let buffer = serializer.into_inner();
            let buffer_slice =
                &buffer[..core::mem::size_of::<<BatteryVoltage as Archive>::Archived>()];
            sensors_file.extend_from_u8(4).await.unwrap();
            sensors_file.extend_from_slice(buffer_slice).await.unwrap();

            buffer
        }
    }
}

#[inline(never)]
pub async fn avionics_main(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) -> ! {
    // Init 1KiB heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let timer = device_manager.timer;
    claim_devices!(device_manager, buzzer);

    let sensors_file_id = unwrap!(fs.create_file(AVIONICS_SENSORS_FILE_TYPE).await.ok());
    let mut sensors_file = unwrap!(fs.open_file_for_write(sensors_file_id).await.ok());
    let log_file_id = unwrap!(fs.create_file(AVIONICS_LOG_FILE_TYPE).await.ok());
    let mut logs_file = unwrap!(fs.open_file_for_write(log_file_id).await.ok());

    let landed = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let sensors_file_should_write_all = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));
    let sensors_file_channel = PubSubChannel::<NoopRawMutex, SensorReading, 200, 1, 1>::new();

    let sensors_file_fut = async {
        let write_interval_ms = 10000.0;
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
                    SensorReading::BatteryVoltage(battery_voltage) => {
                        if battery_voltage.timestamp - last_batt_volt > write_interval_ms {
                            last_batt_volt = battery_voltage.timestamp;
                            buffer = save_sensor_reading(sensor_reading, &mut sensors_file, buffer)
                                .await;
                        }
                    }
                }
            }
        }
    };

    if self_test(device_manager).await {
        buzzer.play(2000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(2000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(3000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(3000, 50.0).await;
    } else {
        buzzer.play(3000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(3000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(2000, 50.0).await;
        timer.sleep(150.0).await;
        buzzer.play(2000, 50.0).await;
    }

    let shared_buzzer_channel = PubSubChannel::<NoopRawMutex, Vec<BuzzerTone, 7>, 2, 1, 1>::new();

    let shared_buzzer_fut = async {
        let mut sub = shared_buzzer_channel.subscriber().unwrap();
        loop {
            let tones = sub.next_message_pure().await;
            for tone in tones {
                if let Some(frequency) = tone.0 {
                    buzzer.play(frequency, tone.1 as f64).await;
                } else {
                    timer.sleep(tone.1 as f64).await;
                }
            }
        }
    };

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
        vlp_phy,
        camera
    );
    vlp_phy.set_output_power(22);
    unwrap!(imu.wait_for_power_on().await);
    unwrap!(imu.reset().await);
    unwrap!(barometer.reset().await);
    unwrap!(meg.reset().await);

    let imu = Mutex::<NoopRawMutex, _>::new(imu);

    let gps_parser = GPSParser::new(timer);

    let cal_info = read_imu_calibration_file(fs).await;

    let radio_tx = Channel::<NoopRawMutex, ApplicationLayerTxPackage, 1>::new();
    let radio_rx = Channel::<NoopRawMutex, ApplicationLayerRxPackage, 3>::new();

    let telemetry_data: BlockingMutex<NoopRawMutex, RefCell<TelemetryData>> =
        BlockingMutex::new(RefCell::new(TelemetryData::default()));

    let flight_core_events: Channel<NoopRawMutex, FlightCoreEvent, 3> = Channel::<NoopRawMutex, FlightCoreEvent, 3>::new();

    let gps_fut = gps_parser.run(&mut gps);

    let arming_state = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(ArmingState {
        hardware_armed: false,
        software_armed: false,
    }));

    let radio_fut = async {
        let mut socket = PVLPMaster::new(PVLP(vlp_phy), timer);

        loop {
            let tx_package = radio_tx.recv().await;
            let rx = socket.tx_and_rx(tx_package).await;
            if let Some(rx_package) = rx {
                radio_rx.send(rx_package).await;
            }
        }
    };

    let main_fut = async {
        // TODO buzzer
        let rocket_upright_acc: BlockingMutex<NoopRawMutex, RefCell<Option<Vector3<f32>>>> =
            BlockingMutex::new(RefCell::new(read_up_right_vector(fs).await));
        let flight_core: Mutex<
            NoopRawMutex,
            Option<FlightCore<Sender<NoopRawMutex, FlightCoreEvent, 3>>>,
        > = Mutex::new(None);

        let mut imu_ticker = Ticker::every(timer, 5.0);
        let imu_channel = PubSubChannel::<NoopRawMutex, IMUReading, 1, 1, 1>::new();
        let mut baro_ticker = Ticker::every(timer, 5.0);
        let baro_channel = PubSubChannel::<NoopRawMutex, BaroReading, 1, 1, 1>::new();
        let mut meg_ticker = Ticker::every(timer, 50.0);
        let mut batt_volt_ticker = Ticker::every(timer, 5.0);

        let pyro1_cont_fut = async {
            let cont = unwrap!(pyro1_cont.read_continuity().await);
            telemetry_data.lock(|d| {
                let mut d = d.borrow_mut();
                d.pyro1_cont = cont
            });

            loop {
                let cont = unwrap!(pyro1_cont.wait_continuity_change().await);
                telemetry_data.lock(|d| {
                    let mut d = d.borrow_mut();
                    d.pyro1_cont = cont
                });
            }
        };

        let pyro2_cont_fut = async {
            let cont = unwrap!(pyro2_cont.read_continuity().await);
            telemetry_data.lock(|d| {
                let mut d = d.borrow_mut();
                d.pyro2_cont = cont
            });

            loop {
                let cont = unwrap!(pyro2_cont.wait_continuity_change().await);
                telemetry_data.lock(|d| {
                    let mut d = d.borrow_mut();
                    d.pyro2_cont = cont
                });
            }
        };

        let imu_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed()) && let Ok(mut imu) = imu.try_lock() {
                    let imu_reading = unwrap!(imu.read().await);
                    sensors_file_channel.publish_immediate(SensorReading::IMU(imu_reading.clone()));
                    imu_channel.publish_immediate(imu_reading);
                }
                imu_ticker.next().await;
            }
        };

        let baro_fut = async {
            loop {
                if arming_state.lock(|s| (*s.borrow()).is_armed()) {
                    let baro_reading = unwrap!(barometer.read().await);
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
                        d.satellites_in_use = nmea.num_of_fix_satellites;
                        d.lat_lon = nmea.lat_lon;
                    });
                    sensors_file_channel.publish_immediate(SensorReading::GPS(nmea));
                }

                timer.sleep(5.0).await;
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
                    let batt_volt_reading = unwrap!(batt_voltmeter.read().await);
                    telemetry_data.lock(|d| {
                        let mut d = d.borrow_mut();
                        d.battery_voltage = batt_volt_reading;
                    });
                    sensors_file_channel.publish_immediate(SensorReading::BatteryVoltage(
                        BatteryVoltage {
                            timestamp: timer.now_mills(),
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
                let new_arming_state = unwrap!(arming_switch.wait_arming_change().await);
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
                            tones.push(BuzzerTone(Some(2000), 500.0)).unwrap();
                            tones.push(BuzzerTone(None, 500.0)).unwrap();
                            tones.push(BuzzerTone(Some(3000), 500.0)).unwrap();
                            shared_buzzer_channel.publish_immediate(tones);
                        } else {
                            let mut tones = Vec::new();
                            tones.push(BuzzerTone(Some(3000), 500.0)).unwrap();
                            tones.push(BuzzerTone(None, 500.0)).unwrap();
                            tones.push(BuzzerTone(Some(2000), 500.0)).unwrap();
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

        let radio_fut = async {
            loop {
                match radio_rx.recv().await {
                    ApplicationLayerRxPackage::VerticalCalibration => {
                        log_info!("Vertical calibration");
                        if !arming_state.lock(|s| (*s.borrow()).is_armed()) {
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
                            acc_sum /= 100.0;
                            rocket_upright_acc.lock(|s| s.borrow_mut().replace(acc_sum));
                            write_up_right_vector(fs, acc_sum).await.unwrap();

                            let mut tones = Vec::new();
                            tones.push(BuzzerTone(Some(2700), 500.0)).unwrap();
                            tones.push(BuzzerTone(None, 250.0)).unwrap();
                            tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                            tones.push(BuzzerTone(None, 150.0)).unwrap();
                            tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                            shared_buzzer_channel.publish_immediate(tones);
                        }
                    }
                    ApplicationLayerRxPackage::ClearStorage => {
                        log_info!("Clearing storage");
                        fs.remove_files(|entry| {
                            !entry.opened
                                && (entry.file_type == AVIONICS_LOG_FILE_TYPE
                                    || entry.file_type == AVIONICS_SENSORS_FILE_TYPE)
                        })
                        .await
                        .unwrap();
                        let mut tones = Vec::new();
                        tones.push(BuzzerTone(Some(2700), 500.0)).unwrap();
                        tones.push(BuzzerTone(None, 250.0)).unwrap();
                        tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                        tones.push(BuzzerTone(None, 150.0)).unwrap();
                        tones.push(BuzzerTone(Some(3700), 50.0)).unwrap();
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
                pyro1_cont_fut,
                pyro2_cont_fut
            );
        }
    };

    let telemetry_fut = async {
        let mut telemetry_ticker = Ticker::every(timer, 3000.0);
        let mut last_telemetry_timestamp = -60_000.0f64;
        loop {
            let should_throttle = !arming_state.lock(|s| (*s.borrow()).is_armed())
                || telemetry_data.lock(|d| d.borrow().avionics_state == AvionicsState::Landed);
            if should_throttle && timer.now_mills() - last_telemetry_timestamp <= 60_000.0 {
                telemetry_ticker.next().await;
                continue;
            }

            let mut telemetry_data = telemetry_data.lock(|d| d.borrow().clone());
            telemetry_data.timestamp = timer.now_mills();
            last_telemetry_timestamp = telemetry_data.timestamp;
            log_info!("Sending telemetry {:?}", telemetry_data);
            radio_tx
                .send(ApplicationLayerTxPackage::Telemetry(telemetry_data))
                .await;
            telemetry_ticker.next().await;
        }
    };

    let camera_ctrl_future = async {
        let mut ticker = Ticker::every(timer, 3000.0);
        let mut is_recording = false;

        loop {
            let should_record = arming_state.lock(|s| (*s.borrow()).is_armed())
                && telemetry_data.lock(|d| d.borrow().avionics_state != AvionicsState::Landed);
            if should_record && !is_recording {
                camera.set_recording(true).await;
                is_recording = true;
            } else if !should_record && is_recording {
                timer.sleep(60_000.0).await;
                camera.set_recording(false).await;
                is_recording = false;
            }
            ticker.next().await;
        }
    };

    let flight_core_event_consumer = async {
        let receiver = flight_core_events.receiver();
        // pyro1: main, pyro2: drogue
        claim_devices!(device_manager, pyro1_ctrl, pyro2_ctrl);
        let debugger = device_manager.debugger.clone();
        loop {
            let event = receiver.recv().await;
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
                    timer.sleep(3000.0);
                    pyro1_ctrl.set_enable(false).await.ok();
                }
                FlightCoreEvent::DeployDrogue => {
                    pyro2_ctrl.set_enable(true).await.ok();
                    timer.sleep(3000.0);
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

    let mut landed_buzzing_ticker = Ticker::every(timer, 5000.0);
    let landed_buzzing_fut = async {
        loop {
            if landed.lock(|s| *s.borrow()) {
                let mut tones = Vec::new();
                tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                tones.push(BuzzerTone(None, 150.0)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                tones.push(BuzzerTone(None, 500.0)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
                tones.push(BuzzerTone(None, 150.0)).unwrap();
                tones.push(BuzzerTone(Some(2700), 50.0)).unwrap();
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
        sensors_file_fut,
        camera_ctrl_future,
    );
    defmt::unreachable!();
}

const MARAUDER_2_FLIGHT_CONFIG: FlightCoreConfig = FlightCoreConfig {
    drogue_chute_minimum_time_ms: 20_000.0,
    drogue_chute_minimum_altitude_agl: 2000.0,
    drogue_chute_delay_ms: 2000.0,
    main_chute_delay_ms: 0.0,
    main_chute_altitude_agl: 365.0, // 1200 ft
};

#[derive(Debug, Clone, Copy, PartialEq)]
struct BuzzerTone(Option<u32>, f32);

struct ArmingState {
    pub hardware_armed: bool,
    pub software_armed: bool,
}

impl ArmingState {
    pub fn is_armed(&self) -> bool {
        self.hardware_armed && self.software_armed
    }
}
