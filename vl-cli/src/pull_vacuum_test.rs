use crate::{
    list_files::list_files, pull_delta_logs::pull_delta_logs, pull_logs::pull_logs,
    reader::BufReaderWrapper, LSArgs, PullVacuumTestArgs,
};
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::file_types::{
        VACUUM_TEST_BARO_LOGGER_TIER_1, VACUUM_TEST_BARO_LOGGER_TIER_2, VACUUM_TEST_LOG_FILE_TYPE,
    },
    driver::{barometer::BaroData, serial::SplitableSerial},
    vacuum_test::{SensorsFF1, SensorsFF2, VacuumTestLoggerReader},
    RpcClient,
};
use std::fs;
use std::vec;
use tokio::fs::File;

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
        pull_logs::<VacuumTestLoggerReader<BufReaderWrapper<File>>>(
            rpc,
            args.save_path.clone(),
            file_id,
            "vacuum_test_log",
        )
        .await?;
    }

    for file_id in baro_tier_1_files {
        pull_delta_logs::<BaroData, SensorsFF1>(
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
        )
        .await?;
    }

    // for file_id in baro_tier_2_files {
    //     pull_delta_logs::<BaroData, SensorsFF2>(
    //         rpc,
    //         args.save_path.clone(),
    //         file_id,
    //         "baro_tier_2",
    //         vec!["pressure".into(), "altitude".into(), "temperature".into()],
    //         |data| {
    //             vec![
    //                 format!("{}", data.pressure),
    //                 format!("{}", data.altitude()),
    //                 format!("{}", data.temperature),
    //             ]
    //         },
    //     )
    //     .await?;
    // }

    Ok(())
}
