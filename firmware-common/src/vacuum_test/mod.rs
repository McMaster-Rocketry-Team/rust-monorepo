use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::signal::Signal;
use futures::join;

use crate::avionics::backup_flight_core::BackupFlightCore;
use crate::avionics::flight_core_event::{FlightCoreEvent, FlightCoreState};
use crate::avionics::flight_profile::FlightProfile;
use crate::common::config_file::ConfigFile;
use crate::common::delta_logger::buffered_logger::BufferedLoggerState;
use crate::common::delta_logger::ring_delta_logger::{RingDeltaLoggerConfig, RingDeltaLoggerState};
use crate::common::file_types::{
    FLIGHT_PROFILE_FILE_TYPE, VACUUM_TEST_BARO_LOGGER, VACUUM_TEST_LOG_FILE_TYPE,
};
use crate::common::ticker::Ticker;
use crate::common::vl_device_manager::prelude::*;
use crate::driver::barometer::BaroData;
use crate::{claim_devices, create_serialized_enum, fixed_point_factory};
use crate::{system_services_type, vl_device_manager_type};

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, defmt::Format)]
pub struct FlightCoreEventLog {
    timestamp: f64,
    event: FlightCoreEvent,
}

create_serialized_enum!(
    VacuumTestLogger,
    VacuumTestLoggerReader,
    VacuumTestLog,
    (0, FlightCoreEventLog)
);

fixed_point_factory!(SensorsFF1, f64, 4.0, 7.0, 0.05);

#[inline(never)]
pub async fn vacuum_test_main(
    device_manager: vl_device_manager_type!(),
    services: system_services_type!(),
) -> ! {
    claim_devices!(device_manager, indicators, barometer);

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

    log_info!("Creating logger");
    let mut log_file_writer = services
        .fs
        .create_file_and_open_for_write(VACUUM_TEST_LOG_FILE_TYPE)
        .await
        .unwrap();
    let mut logger = VacuumTestLogger::new();

    log_info!("Creating baro logger");
    let baro_logger_state = RingDeltaLoggerState::<BaroData, _, _, SensorsFF1, _, _>::new(
        services.fs,
        services.delay.clone(),
        services.clock.clone(),
        RingDeltaLoggerConfig {
            file_type: VACUUM_TEST_BARO_LOGGER,
            seconds_per_segment: 60,
            first_segment_seconds: 30,
            segments_per_ring: 30, // 30 min
        },
    )
    .await
    .unwrap();
    let (baro_logger, mut baro_logger_runner) = baro_logger_state.get_logger_runner();
    let buffered_baro_logger_state = BufferedLoggerState::<_, _, _, 10>::new(baro_logger);
    let (baro_logger, mut baro_logger_buffered_runner) =
        buffered_baro_logger_state.get_logger_runner();

    let flight_core_events = PubSubChannel::<NoopRawMutex, FlightCoreEvent, 3, 1, 1>::new();
    let mut flight_core =
        BackupFlightCore::new(flight_profile, flight_core_events.publisher().unwrap());

    let mut baro_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let baro_fut = async {
        loop {
            baro_ticker.next().await;
            let baro_reading = barometer.read().await.unwrap();
            flight_core.tick(&baro_reading);
            baro_logger.ref_log(baro_reading);
        }
    };

    let flight_core_state_signal = Signal::<NoopRawMutex, FlightCoreState>::new();
    let flight_core_events_sub_fut = async {
        let mut sub = flight_core_events.subscriber().unwrap();
        loop {
            let event = sub.next_message_pure().await;
            match event {
                FlightCoreEvent::ChangeState(state) => {
                    log_info!("change state event: {:?}", event);
                    flight_core_state_signal.signal(state);
                    if state == FlightCoreState::MainChuteDeployed
                        || state == FlightCoreState::DrogueChuteDeployed
                    {
                        services.buzzer_queue.publish(2700, 2000, 150);
                    }
                    logger
                        .write(
                            &mut log_file_writer,
                            &VacuumTestLog::FlightCoreEventLog(FlightCoreEventLog {
                                timestamp: services.clock().now_ms(),
                                event: event.clone(),
                            }),
                        )
                        .await
                        .unwrap();
                    log_file_writer.flush().await.unwrap();
                }
                _ => {
                    // noop
                }
            }
        }
    };

    let indicators_fut = async {
        let mut state = flight_core_state_signal.wait().await;
        loop {
            let indicator_fut = async {
                match state {
                    FlightCoreState::DrogueChuteDeployed => {
                        indicators.run([], [], [250, 250]).await;
                    }
                    FlightCoreState::MainChuteDeployed => {
                        indicators.run([], [250, 250], []).await;
                    }
                    FlightCoreState::Landed => {
                        indicators.run([], [250, 250], [0, 250, 250, 0]).await;
                    }
                    _ => {
                        indicators.run([], [50, 950], []).await;
                    }
                }
            };

            let wait_signal_fut = flight_core_state_signal.wait();

            match select(indicator_fut, wait_signal_fut).await {
                Either::First(_) => {}
                Either::Second(new_state) => {
                    state = new_state;
                }
            };
        }
    };

    services.buzzer_queue.publish(2000, 50, 150);
    services.buzzer_queue.publish(2000, 50, 150);

    #[allow(unused_must_use)]
    {
        join!(
            baro_logger_runner.run(),
            baro_logger_buffered_runner.run(),
            baro_fut,
            flight_core_events_sub_fut,
            indicators_fut
        );
    }
    log_unreachable!();
}
