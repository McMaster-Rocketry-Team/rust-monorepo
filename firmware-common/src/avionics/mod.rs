use defmt::unwrap;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Sender},
};
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

    claim_devices!(device_manager, arming_switch, imu);
    unwrap!(imu.wait_for_power_on().await);
    unwrap!(imu.reset().await);

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
        let mut last_arming_state = unwrap!(arming_switch.read_arming().await);
        let mut ticker = Ticker::every(timer, 5.0);
        let mut rocket_upright_acc: Option<Vector3<f32>> = None;
        let mut flight_core: Option<
            FlightCore<Sender<CriticalSectionRawMutex, FlightCoreEvent, 3>>,
        > = None;

        loop {
            let arming_state = unwrap!(arming_switch.read_arming().await);
            if arming_state != last_arming_state {
                if arming_state {
                    if let Some(rocket_upright_acc) = rocket_upright_acc {
                        let variances = if let Some(cal_info) = &cal_info {
                            Variances::from_imu_cal_info(cal_info, 2.0)
                        } else {
                            Variances::default()
                        };
                        flight_core = Some(FlightCore::new(
                            MARAUDER_2_FLIGHT_CONFIG,
                            flight_core_events.sender(),
                            rocket_upright_acc,
                            variances,
                        ));

                        buzzer.play(2000, 500.0).await;
                        timer.sleep(500.0).await;
                        buzzer.play(3000, 500.0).await;
                    } else {
                        buzzer.play(3000, 500.0).await;
                        timer.sleep(500.0).await;
                        buzzer.play(2000, 500.0).await;
                    }
                } else {
                    flight_core = None;
                }
            }

            if arming_state {
                if let Some(ref mut flight_core) = &mut flight_core {
                    let mut imu_reading = unwrap!(imu.read().await);
                    if let Some(cal_info) = &cal_info {
                        imu_reading = cal_info.apply_calibration(&imu_reading);
                    }
                    // TODO gps and baro
                    let sensor_snapshot = PartialSensorSnapshot {
                        timestamp: imu_reading.timestamp,
                        imu_reading: imu_reading,
                        gps_location: None,
                        baro_reading: None,
                    };

                    flight_core.tick(sensor_snapshot);
                }
            } else {
                while let Ok(package) = radio_rx.try_recv() {
                    match package {
                        ApplicationLayerRxPackage::VerticalCalibration => {
                            log_info!("Vertical calibration");
                            let mut ticker = Ticker::every(timer, 1.0);
                            let mut acc_sum = Vector3::<f32>::zeros();
                            for _ in 0..100 {
                                let mut reading = unwrap!(imu.read().await);
                                if let Some(cal_info) = &cal_info {
                                    reading = cal_info.apply_calibration(&reading);
                                }
                                acc_sum += Vector3::from(reading.acc);
                                ticker.next().await;
                            }
                            rocket_upright_acc = Some(acc_sum / 100.0);

                            buzzer.play(2700, 500.0).await;
                            timer.sleep(250.0).await;
                            buzzer.play(2700, 50.0).await;
                            timer.sleep(150.0).await;
                            buzzer.play(2700, 50.0).await;
                        }
                        ApplicationLayerRxPackage::SoftArming(_) => {
                            // todo
                        }
                    }
                }
            }

            last_arming_state = arming_state;
            ticker.next().await;
        }
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
