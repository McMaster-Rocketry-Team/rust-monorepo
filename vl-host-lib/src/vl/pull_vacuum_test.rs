use anyhow::Result;
use firmware_common::{
    common::file_types::{VACUUM_TEST_BARO_LOGGER, VACUUM_TEST_LOG_FILE_TYPE},
    driver::{barometer::BaroData, serial::SplitableSerial},
    vacuum_test::{SensorsFF1, VacuumTestLoggerReader},
    CommonRPCTrait,
};
use std::vec;
use std::{fs, path::PathBuf};
use tokio::fs::File;

use crate::common::{
    list_files, pull_delta_readings, pull_serialized_enums, readers::BufReaderWrapper,
};

pub async fn pull_vacuum_test<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: PathBuf,
) -> Result<()> {
    fs::create_dir_all(&save_folder)?;

    let log_files = list_files(rpc, Some(VACUUM_TEST_LOG_FILE_TYPE)).await?;

    let mut combined_logs_path = save_folder.clone();
    combined_logs_path.push("combined.vacuum_test_log.log");
    let mut combined_logs_writer =
        tokio::io::BufWriter::new(File::create(combined_logs_path).await?);
    for file_id in log_files {
        pull_serialized_enums::<_, VacuumTestLoggerReader<BufReaderWrapper<File>>>(
            rpc,
            save_folder.clone(),
            file_id,
            "vacuum_test_log",
            &mut combined_logs_writer,
        )
        .await?;
    }

    pull_delta_readings::<_, BaroData, SensorsFF1>(
        rpc,
        save_folder.clone(),
        VACUUM_TEST_BARO_LOGGER,
        "baro",
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

    Ok(())
}
