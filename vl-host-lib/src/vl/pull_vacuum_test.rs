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
use tokio::{fs::File, io::AsyncWriteExt};

use crate::common::{
    extend_path, pull_file::pull_files, pull_serialized_enums::read_serialized_enums,
    read_delta_readings::read_delta_readings, readers::BufReaderWrapper,
    sensor_reading_csv_writer::SensorReadingCSVWriter,
};

pub async fn pull_vacuum_test<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: PathBuf,
) -> Result<()> {
    fs::create_dir_all(&save_folder)?;

    // logs
    let log_files = pull_files(
        rpc,
        VACUUM_TEST_LOG_FILE_TYPE,
        "vacuum_test_log",
        "vle",
        &save_folder,
    )
    .await?;
    let stream =
        read_serialized_enums::<VacuumTestLoggerReader<BufReaderWrapper<File>>>(log_files).await?;
    pin_mut!(stream);

    let mut logs_writer = tokio::io::BufWriter::new(
        File::create(extend_path(&save_folder, "vacuum_test_log.log")).await?,
    );
    while let Some(log) = stream.next().await {
        logs_writer
            .write_all(format!("{:?}\n", log).as_bytes())
            .await?;
    }
    logs_writer.flush().await?;

    // baro readings
    let baro_files = pull_files(rpc, VACUUM_TEST_BARO_LOGGER, "baro", "vldr", &save_folder).await?;
    let stream = read_delta_readings::<BaroData, SensorsFF1>(baro_files).await?;

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
