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
use vlfs::{Crc, Flash, VLFS};

use crate::{
    avionics::{
        flight_core::{FlightCore, Variances},
        flight_core_event::FlightCoreEvent,
    },
    claim_devices,
    common::{
        device_manager::prelude::*, files::CALIBRATION_FILE_TYPE,
        imu_calibration_file::read_imu_calibration_file, sensor_snapshot::PartialSensorSnapshot,
        ticker::Ticker,
    },
    device_manager_type,
    driver::{
        barometer::BaroReading,
        debugger::{ApplicationLayerRxPackage, ApplicationLayerTxPackage, RadioApplicationClient},
        gps::GPS,
        indicator::Indicator,
        timer::Timer,
    },
};

use self::flight_core::Config as FlightCoreConfig;

pub mod baro_reading_filter;
pub mod flight_core;
pub mod flight_core_event;

#[inline(never)]
pub async fn avionics_main(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) -> ! {
    let timer = device_manager.timer;
    claim_devices!(device_manager, buzzer);

    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(2000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;
    buzzer.play(3000, 50.0).await;
    timer.sleep(150.0).await;

    let buzzer = Mutex::<NoopRawMutex, _>::new(buzzer);

    claim_devices!(device_manager, arming_switch, imu, barometer);
    unwrap!(imu.wait_for_power_on().await);
    unwrap!(imu.reset().await);
    unwrap!(barometer.reset().await);

    let imu = Mutex::<NoopRawMutex, _>::new(imu);

    let cal_info = read_imu_calibration_file(fs).await;

    fs.find_file_by_type(CALIBRATION_FILE_TYPE).await;

    let radio = device_manager.get_radio_application_layer().await;

    let radio_tx = Channel::<CriticalSectionRawMutex, ApplicationLayerTxPackage, 3>::new();
    let radio_rx = Channel::<CriticalSectionRawMutex, ApplicationLayerRxPackage, 3>::new();

    let flight_core_events = Channel::<CriticalSectionRawMutex, FlightCoreEvent, 3>::new();

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

        let imu_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) && let Ok(mut imu) = imu.try_lock() {
                    let imu_reading = unwrap!(imu.read().await);
                    imu_channel.publish_immediate(imu_reading);
                }
                imu_ticker.next().await;
            }
        };

        let baro_fut = async {
            loop {
                if arming_state.lock(|s| *s.borrow()) {
                    let baro_reading = unwrap!(barometer.read().await);
                    baro_channel.publish_immediate(baro_reading);
                }
                baro_ticker.next().await;
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
                if arming_state.lock(|s| *s.borrow()) &&
                let Ok(mut flight_core) = flight_core.try_lock() &&
                let Some(flight_core)= flight_core.as_mut() {
                    let imu_reading = imu_sub.next_message_pure().await;
                    let baro_reading = baro_sub.try_next_message_pure();
                    // TODO gps
                    let sensor_snapshot = PartialSensorSnapshot {
                        timestamp: imu_reading.timestamp,
                        imu_reading: imu_reading,
                        gps_location: None,
                        baro_reading: baro_reading,
                    };

                    flight_core.tick(sensor_snapshot);
                }else{
                    timer.sleep(5.0).await;
                }
            }
        };

        join!(imu_fut, baro_fut, arming_fut, radio_fut, flight_core_fut);
    };

    let flight_core_event_consumer = async {
        let receiver = flight_core_events.receiver();
        // TODO check again: pyro1: main, pyro2: drogue
        claim_devices!(device_manager, pyro1_ctrl, pyro2_ctrl);
        loop {
            let event = receiver.recv().await;
            match event {
                FlightCoreEvent::CriticalError => {
                    claim_devices!(device_manager, sys_reset);
                    sys_reset.reset();
                }
                FlightCoreEvent::Ignition => {
                    // TODO cameras
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
                }
                FlightCoreEvent::DidNotReachMinApogee => {
                    // noop
                }
            }
        }
    };

    join!(radio_fut, main_fut, flight_core_event_consumer);
    defmt::unreachable!();
}

const MARAUDER_2_FLIGHT_CONFIG: FlightCoreConfig = FlightCoreConfig {
    drogue_chute_minimum_time_ms: 20_000.0,
    drogue_chute_minimum_altitude_agl: 2000.0,
    drogue_chute_delay_ms: 1000.0,
    main_chute_delay_ms: 1000.0,
    main_chute_altitude_agl: 457.0, // 1500 ft
};
