use crate::{
    list_files::list_files, pull_file::pull_file, reader::VecReader, LSArgs, PullArgs,
    PullVacuumTestArgs,
};
use anyhow::Result;
use either::Either;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::{
        delta_logger::{delta_logger::UnixTimestampLog, prelude::DeltaLoggerReader},
        file_types::{
            VACUUM_TEST_BARO_LOGGER_TIER_1, VACUUM_TEST_BARO_LOGGER_TIER_2,
            VACUUM_TEST_LOG_FILE_TYPE,
        },
        sensor_reading::SensorReading,
    },
    driver::{barometer::BaroData, serial::SplitableSerial, timestamp::BootTimestamp},
    vacuum_test::{SensorsFF1, VacuumTestLoggerReader},
    RpcClient,
};
use map_range::MapRange;
use std::io::Write;
use std::{fs, ops::Range};

pub async fn pull_vacuum_test(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    args: PullVacuumTestArgs,
) -> Result<()> {
    fs::create_dir_all(&args.save_path)?;

    let log_files = list_files(
        rpc,
        LSArgs {
            file_type: Some(VACUUM_TEST_LOG_FILE_TYPE.0),
        },
    )
    .await?;

    let baro_tier_1_files = list_files(
        rpc,
        LSArgs {
            file_type: Some(VACUUM_TEST_BARO_LOGGER_TIER_1.0),
        },
    )
    .await?;

    let baro_tier_2_files = list_files(
        rpc,
        LSArgs {
            file_type: Some(VACUUM_TEST_BARO_LOGGER_TIER_2.0),
        },
    )
    .await?;

    for file_id in log_files {
        let mut path = args.save_path.clone();
        path.push(format!("{}.vacuum_test_log.voidlake", file_id));
        let content = pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;

        let reader = VecReader::new(content);
        let mut reader = VacuumTestLoggerReader::new(reader);

        let mut path = args.save_path.clone();
        path.push(format!("{}.vacuum_test_log.log", file_id));
        let mut file = fs::File::create(&path)?;
        while let Some(log) = reader.read_next().await.unwrap() {
            file.write_all(format!("{:?}\n", log).as_bytes())?;
        }
    }

    for file_id in baro_tier_1_files {
        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_1.voidlake", file_id));
        let content = pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;

        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_1.csv", file_id));
        let reader = VecReader::new(content);
        let mut reader = DeltaLoggerReader::<BaroData, _, SensorsFF1>::new(reader);
        let mut csv_writer = csv::Writer::from_path(&path)?;
        csv_writer.write_record(&[
            "boot timestamp",
            "unix timestamp",
            "pressure",
            "temperature",
        ])?;

        let mut last_unix_timestamp: Option<UnixTimestampLog> = None;
        let mut last_ranges: Option<(Range<f64>, Range<f64>)> = None;
        let mut readings_buffer: Vec<SensorReading<BootTimestamp, BaroData>> = Vec::new();
        while let Some(reading) = reader.read().await.unwrap() {
            match reading {
                Either::Left(reading) => {
                    readings_buffer.push(reading);
                }
                Either::Right(new_unix_time_log) => {
                    if let Some(last_unix_timestamp) = last_unix_timestamp.take() {
                        let unix_timestamp_range =
                            last_unix_timestamp.unix_timestamp..new_unix_time_log.unix_timestamp;
                        let boot_timestamp_range =
                            last_unix_timestamp.boot_timestamp..new_unix_time_log.boot_timestamp;
                        last_ranges =
                            Some((unix_timestamp_range.clone(), boot_timestamp_range.clone()));

                        for reading in readings_buffer.iter() {
                            let unix_timestamp = reading.timestamp.map_range(
                                boot_timestamp_range.clone(),
                                unix_timestamp_range.clone(),
                            );
                            csv_writer.write_record(&[
                                format!("{}", reading.timestamp),
                                format!("{}", unix_timestamp),
                                format!("{}", reading.data.pressure),
                                format!("{}", reading.data.temperature),
                            ])?;
                        }
                        readings_buffer.clear();
                    } else {
                        for reading in readings_buffer.iter() {
                            csv_writer.write_record(&[
                                format!("{}", reading.timestamp),
                                "".into(),
                                format!("{}", reading.data.pressure),
                                format!("{}", reading.data.temperature),
                            ])?;
                        }
                        readings_buffer.clear();
                    }
                    last_unix_timestamp = Some(new_unix_time_log);
                }
            }
        }
        for reading in readings_buffer.iter() {
            let unix_timestamp =
                if let Some((boot_timestamp_range, unix_timestamp_range)) = last_ranges.clone() {
                    let unix_timestamp = reading
                        .timestamp
                        .map_range(boot_timestamp_range, unix_timestamp_range);
                    format!("{}", unix_timestamp)
                } else {
                    "".into()
                };

            csv_writer.write_record(&[
                format!("{}", reading.timestamp),
                unix_timestamp,
                format!("{}", reading.data.pressure),
                format!("{}", reading.data.temperature),
            ])?;
        }
        csv_writer.flush()?;
    }

    for file_id in baro_tier_2_files {
        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_2.voidlake", file_id));
        let content = pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;

        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_2.csv", file_id));
        let reader = VecReader::new(content);
        let mut reader = DeltaLoggerReader::<BaroData, _, SensorsFF1>::new(reader);
        let mut csv_writer = csv::Writer::from_path(&path)?;
        csv_writer.write_record(&["timestamp", "pressure", "temperature"])?;
        // while let Some(reading) = reader.read().await.unwrap() {
        //     csv_writer.write_record(&[
        //         format!("{}", reading.timestamp),
        //         format!("{}", reading.data.pressure),
        //         format!("{}", reading.data.temperature),
        //     ])?;
        // }
        csv_writer.flush()?;
    }

    Ok(())
}
