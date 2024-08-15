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
    extend_path, parse_delta_readings::parse_delta_readings,
    parse_serialized_enums::parse_serialized_enums, pull_file::pull_files,
    readers::BufReaderWrapper, SensorReadingCSVWriter,
};

pub async fn pull_vacuum_test<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: &PathBuf,
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

    let mut logs_writer = tokio::io::BufWriter::new(
        File::create(extend_path(&save_folder, "vacuum_test_log.log")).await?,
    );
    for file_path in log_files {
        let stream =
            parse_serialized_enums::<VacuumTestLoggerReader<BufReaderWrapper<File>>>(file_path)
                .await?;
        pin_mut!(stream);

        while let Some(log) = stream.next().await {
            logs_writer
                .write_all(format!("{:?}\n", log).as_bytes())
                .await?;
        }
    }
    logs_writer.flush().await?;

    // baro readings
    let baro_files = pull_files(rpc, VACUUM_TEST_BARO_LOGGER, "baro", "vldr", &save_folder).await?;

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
    for file_path in baro_files {
        let stream = parse_delta_readings::<BaroData, SensorsFF1>(file_path).await?;
        csv_writer.write_all(stream).await?;
    }
    csv_writer.flush()?;

    Ok(())
}
