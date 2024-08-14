use crate::{
    list_files::list_files, pull_delta_logs::pull_delta_logs, pull_logs::pull_logs,
    reader::BufReaderWrapper, LSArgs, PullDataArgs,
};
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::file_types::{VACUUM_TEST_BARO_LOGGER, VACUUM_TEST_LOG_FILE_TYPE},
    driver::{barometer::BaroData, serial::SplitableSerial},
    vacuum_test::{SensorsFF1, VacuumTestLoggerReader},
    CommonRPCTrait,
};
use std::fs;
use std::vec;
use tokio::fs::File;

pub async fn pull_vacuum_test<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    args: PullDataArgs,
) -> Result<()> {
    fs::create_dir_all(&args.save_path)?;

    let log_files = list_files(
        rpc,
        LSArgs {
            file_type: Some(VACUUM_TEST_LOG_FILE_TYPE),
        },
    )
    .await?;

    let baro_files = list_files(
        rpc,
        LSArgs {
            file_type: Some(VACUUM_TEST_BARO_LOGGER),
        },
    )
    .await?;

    let mut combined_logs_path = args.save_path.clone();
    combined_logs_path.push("combined.vacuum_test_log.log");
    let mut combined_logs_writer =
        tokio::io::BufWriter::new(File::create(combined_logs_path).await?);
    for file_id in log_files {
        pull_logs::<_,VacuumTestLoggerReader<BufReaderWrapper<File>>>(
            rpc,
            args.save_path.clone(),
            file_id,
            "vacuum_test_log",
            &mut combined_logs_writer,
        )
        .await?;
    }

    let mut combined_baro_tier_1_csv_path = args.save_path.clone();
    combined_baro_tier_1_csv_path.push(format!("combined.baro_tier_1.csv"));
    let mut combined_baro_tier_1_csv_writer =
        csv::Writer::from_path(&combined_baro_tier_1_csv_path)?;
    combined_baro_tier_1_csv_writer.write_record(&[
        "boot timestamp",
        "unix timestamp",
        "pressure",
        "altitude",
        "temperature",
    ])?;
    for file_id in baro_files {
        pull_delta_logs::<_,BaroData, SensorsFF1>(
            rpc,
            args.save_path.clone(),
            file_id,
            "baro_tier_1",
            vec!["pressure".into(), "altitude".into(), "temperature".into()],
            |data| {
                vec![
                    format!("{}", data.pressure),
                    format!("{}", data.altitude()),
                    format!("{}", data.temperature),
                ]
            },
            &mut combined_baro_tier_1_csv_writer,
        )
        .await?;
    }

    Ok(())
}
