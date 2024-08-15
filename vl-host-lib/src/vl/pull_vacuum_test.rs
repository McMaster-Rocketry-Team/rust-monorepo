use anyhow::Result;
use firmware_common::{
    common::file_types::{VACUUM_TEST_BARO_LOGGER, VACUUM_TEST_LOG_FILE_TYPE},
    driver::{barometer::BaroData, serial::SplitableSerial},
    vacuum_test::{SensorsFF1, VacuumTestLoggerReader},
    CommonRPCTrait,
};
use futures_util::{pin_mut, StreamExt};
use std::vec;
use std::{fs, path::PathBuf};
use tokio::fs::File;

use crate::common::{
    extend_path, list_files, pull_delta_readings::pull_delta_readings,
    pull_serialized_enums::pull_serialized_enums, readers::BufReaderWrapper,
    sensor_reading_csv_writer::SensorReadingCSVWriter,
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

    let stream = pull_delta_readings::<_, BaroData, SensorsFF1>(
        rpc,
        save_folder.clone(),
        VACUUM_TEST_BARO_LOGGER,
        "baro",
    )
    .await?;

    let mut csv_writer = SensorReadingCSVWriter::new(
        &extend_path(&save_folder, "baro.csv"),
        &["pressure", "altitude", "temperature"],
        |data: BaroData| {
            vec![
                format!("{}", data.pressure),
                format!("{}", data.altitude()),
                format!("{}", data.temperature),
            ]
        },
    )
    .unwrap();
    csv_writer.write_all(stream).await?;
    csv_writer.flush()?;

    Ok(())
}
