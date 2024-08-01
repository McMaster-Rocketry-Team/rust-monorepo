use crate::{
    list_files::list_files, pull_file::pull_file, reader::VecReader, LSArgs, PullArgs,
    PullVacuumTestArgs,
};
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::{
        delta_logger::prelude::DeltaLoggerReader,
        file_types::{
            VACUUM_TEST_BARO_LOGGER_TIER_1, VACUUM_TEST_BARO_LOGGER_TIER_2,
            VACUUM_TEST_LOG_FILE_TYPE,
        },
    },
    driver::{barometer::BaroData, serial::SplitableSerial, timestamp::BootTimestamp},
    vacuum_test::{SensorsFF1, VacuumTestLoggerReader},
    RpcClient,
};
use std::fs;
use std::io::Write;

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
        let mut reader = DeltaLoggerReader::<BootTimestamp, BaroData, _, SensorsFF1>::new(reader);
        let mut csv_writer = csv::Writer::from_path(&path)?;
        csv_writer.write_record(&["timestamp", "pressure", "temperature"])?;
        while let Some(reading) = reader.read().await.unwrap() {
            csv_writer.write_record(&[
                format!("{}", reading.timestamp),
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
        let mut reader = DeltaLoggerReader::<BootTimestamp, BaroData, _, SensorsFF1>::new(reader);
        let mut csv_writer = csv::Writer::from_path(&path)?;
        csv_writer.write_record(&["timestamp", "pressure", "temperature"])?;
        while let Some(reading) = reader.read().await.unwrap() {
            csv_writer.write_record(&[
                format!("{}", reading.timestamp),
                format!("{}", reading.data.pressure),
                format!("{}", reading.data.temperature),
            ])?;
        }
        csv_writer.flush()?;
    }

    Ok(())
}
