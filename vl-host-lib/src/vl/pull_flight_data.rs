use anyhow::Result;
use firmware_common::{
    avionics::{SensorsFF1, SensorsFF2},
    common::file_types::{AVIONICS_BARO_LOGGER_TIER_1, AVIONICS_LOW_G_IMU_LOGGER_TIER_1},
    driver::{barometer::BaroData, imu::IMUData, serial::SplitableSerial},
    CommonRPCTrait,
};
use std::vec;
use std::{fs, path::PathBuf};

use crate::common::{
    extend_path, parse_delta_readings::parse_delta_readings, pull_file::pull_files,
    SensorReadingCSVWriter,
};

pub async fn pull_flight_data<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: &PathBuf,
) -> Result<()> {
    fs::create_dir_all(&save_folder)?;

    // baro readings
    let baro_files = pull_files(
        rpc,
        AVIONICS_BARO_LOGGER_TIER_1,
        "baro_tier_1",
        "vldr",
        &save_folder,
    )
    .await?;

    let mut csv_writer = SensorReadingCSVWriter::new(
        &extend_path(&save_folder, "baro_tier_1.csv"),
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

    let baro_files = pull_files(
        rpc,
        AVIONICS_BARO_LOGGER_TIER_1,
        "baro_tier_2",
        "vldr",
        &save_folder,
    )
    .await?;

    let mut csv_writer = SensorReadingCSVWriter::new(
        &extend_path(&save_folder, "baro_tier_2.csv"),
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
        let stream = parse_delta_readings::<BaroData, SensorsFF2>(file_path).await?;
        csv_writer.write_all(stream).await?;
    }
    csv_writer.flush()?;

    // log g imu readings
    let imu_files = pull_files(
        rpc,
        AVIONICS_LOW_G_IMU_LOGGER_TIER_1,
        "low_g_imu",
        "vldr",
        &save_folder,
    )
    .await?;
    let mut csv_writer = SensorReadingCSVWriter::new(
        &extend_path(&save_folder, "low_g_imu.csv"),
        &["acc_x", "acc_y", "acc_z", "gyro_x", "gyro_y", "gyro_z"],
        |data: IMUData| {
            vec![
                format!("{}", data.acc[0]),
                format!("{}", data.acc[1]),
                format!("{}", data.acc[2]),
                format!("{}", data.gyro[0]),
                format!("{}", data.gyro[1]),
                format!("{}", data.gyro[2]),
            ]
        },
    )
    .unwrap();
    for file_path in imu_files {
        let stream = parse_delta_readings::<IMUData, SensorsFF1>(file_path).await?;
        csv_writer.write_all(stream).await?;
    }
    csv_writer.flush()?;

    Ok(())
}
