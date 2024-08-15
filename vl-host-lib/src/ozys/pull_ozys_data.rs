use anyhow::Result;
use firmware_common::{
    common::file_types::{SG_BATTERY_LOGGER, SG_READINGS},
    driver::{
        adc::{ADCData, Volt},
        serial::SplitableSerial,
    },
    strain_gauges::mid_prio::BatteryFF,
    CommonRPCTrait,
};
use futures_util::{pin_mut, StreamExt};
use std::{array, vec};
use std::{fs, path::PathBuf};

use crate::common::{
    extend_path, pull_file::pull_files, read_delta_readings::read_delta_readings,
    SensorReadingCSVWriter,
};

use super::{parse_sg_data::parse_sg_data, sg_data_csv_writer::SGCSVWriter};

pub async fn pull_ozys_data<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: &PathBuf,
) -> Result<()> {
    fs::create_dir_all(&save_folder)?;

    // battery readings
    let battery_files = pull_files(rpc, SG_BATTERY_LOGGER, "battery", "vldr", &save_folder).await?;
    let stream = read_delta_readings::<ADCData<Volt>, BatteryFF>(battery_files).await?;
    let mut csv_writer = SensorReadingCSVWriter::new(
        &extend_path(&save_folder, "battery.csv"),
        &["voltage"],
        |data: ADCData<Volt>| vec![format!("{}", data.value)],
    )
    .unwrap();
    csv_writer.write_all(stream).await?;
    csv_writer.flush()?;

    // sg readings
    let mut csv_writers: [SGCSVWriter; 4] =
        array::from_fn(|i| SGCSVWriter::new(save_folder, i as u8).unwrap());
    let sg_reading_files = pull_files(rpc, SG_READINGS, "sg", "vle", &save_folder).await?;
    for file_path in sg_reading_files {
        let stream = parse_sg_data(file_path).await?;
        pin_mut!(stream);
        while let Some((reading, unix_timestamp)) = stream.next().await {
            let writer = &mut csv_writers[reading.sg_i as usize];
            writer.write(reading, unix_timestamp)?;
        }
    }
    for writer in &mut csv_writers {
        writer.flush()?;
    }

    Ok(())
}
