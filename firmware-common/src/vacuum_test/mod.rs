use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use futures::join;

use crate::avionics::backup_flight_core::BackupFlightCore;
use crate::avionics::flight_core_event::{ArchivedFlightCoreEvent, FlightCoreEvent};
use crate::avionics::flight_profile::FlightProfile;
use crate::common::config_file::ConfigFile;
use crate::common::delta_logger::buffered_tiered_ring_delta_logger::BufferedTieredRingDeltaLogger;
use crate::common::delta_logger::prelude::{RingDeltaLoggerConfig, TieredRingDeltaLogger};
use crate::common::device_manager::prelude::*;
use crate::common::file_types::{
    FLIGHT_PROFILE_FILE_TYPE, VACUUM_TEST_BARO_LOGGER_TIER_1, VACUUM_TEST_BARO_LOGGER_TIER_2,
    VACUUM_TEST_LOG_FILE_TYPE,
};
use crate::common::ticker::Ticker;
use crate::driver::barometer::BaroData;
use crate::driver::timestamp::BootTimestamp;
use crate::{claim_devices, create_serialized_enum, fixed_point_factory};
use crate::{device_manager_type, system_services_type};

create_serialized_enum!(
    VacuumTestLogger,
    VacuumTestLoggerReader,
    VacuumTestLog,
    (0, FlightCoreEvent)
);

#[inline(never)]
pub async fn vacuum_test_main(
    device_manager: device_manager_type!(),
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
                .await
        };

    log_info!("Creating logger");
    let log_file_writer = services
        .fs
        .create_file_and_open_for_write(VACUUM_TEST_LOG_FILE_TYPE)
        .await
        .unwrap();
    let mut logger = VacuumTestLogger::new(log_file_writer);

    fixed_point_factory!(SensorsFF1, f64, 4.9, 7.0, 0.05);
    fixed_point_factory!(SensorsFF2, f64, 199.0, 210.0, 0.5);
    log_info!("Creating baro logger");
    let baro_logger = BufferedTieredRingDeltaLogger::<BootTimestamp, BaroData, 40>::new();
    let baro_logger_fut = baro_logger.run(
        SensorsFF1,
        SensorsFF2,
        TieredRingDeltaLogger::new(
            services.fs,
            (
                RingDeltaLoggerConfig {
                    file_type: VACUUM_TEST_BARO_LOGGER_TIER_1,
                    seconds_per_segment: 5 * 60,
                    first_segment_seconds: 30,
                    segments_per_ring: 6, // 30 min
                },
                RingDeltaLoggerConfig {
                    file_type: VACUUM_TEST_BARO_LOGGER_TIER_2,
                    seconds_per_segment: 30 * 60,
                    first_segment_seconds: 60,
                    segments_per_ring: 10, // 5 hours
                },
            ),
            services.delay.clone(),
            services.clock.clone(),
        )
        .await
        .unwrap(),
    );

    let flight_core_events = Channel::<NoopRawMutex, FlightCoreEvent, 3>::new();
    let mut flight_core = BackupFlightCore::new(flight_profile, flight_core_events.sender());

    let mut baro_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
    let baro_fut = async {
        loop {
            baro_ticker.next().await;
            let baro_reading = barometer.read().await.unwrap();
            flight_core.tick(&baro_reading);
            baro_logger.log(baro_reading);
        }
    };

    let flight_core_events_sub_fut = async {
        loop {
            let event = flight_core_events.receive().await;
            logger
                .write(&VacuumTestLog::FlightCoreEvent(event))
                .await
                .unwrap();
            logger.flush().await.unwrap();
        }
    };

    join!(baro_logger_fut, baro_fut, flight_core_events_sub_fut);
    log_unreachable!();
}
