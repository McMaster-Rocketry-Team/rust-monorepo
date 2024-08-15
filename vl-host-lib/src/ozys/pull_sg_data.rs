use anyhow::Result;
use firmware_common::{
    common::file_types::{SG_BATTERY_LOGGER, SG_READINGS, VACUUM_TEST_BARO_LOGGER, VACUUM_TEST_LOG_FILE_TYPE},
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

pub async fn pull_sg_data<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: PathBuf,
) -> Result<()> {
    fs::create_dir_all(&save_folder)?;

    let battery_files = list_files(rpc, Some(SG_BATTERY_LOGGER)).await?;

    let sg_reading_files = list_files(rpc, Some(SG_READINGS)).await?;

    Ok(())
}