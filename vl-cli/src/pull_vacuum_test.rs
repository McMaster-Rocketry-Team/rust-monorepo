use crate::{list_files::list_files, pull_file::pull_file, LSArgs, PullArgs, PullVacuumTestArgs};
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::file_types::{
        VACUUM_TEST_BARO_LOGGER_TIER_1, VACUUM_TEST_BARO_LOGGER_TIER_2, VACUUM_TEST_LOG_FILE_TYPE,
    },
    driver::serial::SplitableSerial,
    RpcClient,
};
use std::fs;

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
        pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;
    }

    for file_id in baro_tier_1_files {
        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_1.voidlake", file_id));
        pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;
    }

    for file_id in baro_tier_2_files {
        let mut path = args.save_path.clone();
        path.push(format!("{}.baro_tier_2.voidlake", file_id));
        pull_file(
            rpc,
            PullArgs {
                file_id,
                host_path: path,
            },
        )
        .await?;
    }

    Ok(())
}
