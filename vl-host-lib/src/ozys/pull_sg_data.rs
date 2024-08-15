use anyhow::Result;
use firmware_common::{
    common::file_types::{SG_BATTERY_LOGGER, SG_READINGS},
    driver::{
        adc::{ADCData, Volt},
        serial::SplitableSerial,
    },
    strain_gauges::{
        mid_prio::{BatteryFF, SGReadingLog, SGReadingLoggerReader},
        ProcessedSGReading,
    },
    CommonRPCTrait,
};
use futures_util::{pin_mut, StreamExt};
use std::{array, vec};
use std::{fs, path::PathBuf};
use tokio::fs::File;

use crate::common::{
    extend_path, pull_file::pull_files, pull_serialized_enums::read_serialized_enums,
    read_delta_readings::read_delta_readings, readers::BufReaderWrapper,
    sensor_reading_csv_writer::SensorReadingCSVWriter, unix_timestamp_lut::UnixTimestampLUT,
};

pub async fn pull_sg_data<S: SplitableSerial>(
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
    let sg_reading_files = pull_files(rpc, SG_READINGS, "sg", "vle", &save_folder).await?;
    {
        // Pass 1: get all the unix timestamps
        let mut timestamp_lut = UnixTimestampLUT::new();
        let stream = read_serialized_enums::<SGReadingLoggerReader<BufReaderWrapper<File>>>(
            sg_reading_files.clone(),
        )
        .await?;
        pin_mut!(stream);
        while let Some(reading) = stream.next().await {
            if let SGReadingLog::UnixTimestampLog(log) = reading {
                timestamp_lut.add_timestamp(log.boot_timestamp, log.unix_timestamp)
            }
        }
        timestamp_lut.sort_timestamps();

        // Pass 2
        let mut csv_writers: [SGCSVWriter; 4] =
            array::from_fn(|i| SGCSVWriter::new(save_folder, i as u8, &timestamp_lut).unwrap());
        let stream = read_serialized_enums::<SGReadingLoggerReader<BufReaderWrapper<File>>>(
            sg_reading_files,
        )
        .await?;
        pin_mut!(stream);
        while let Some(reading) = stream.next().await {
            if let SGReadingLog::ProcessedSGReading(reading) = reading {
                let writer = &mut csv_writers[reading.sg_i as usize];
                writer.write(reading)?;
            }
        }

        for writer in &mut csv_writers {
            writer.flush()?;
        }
    }

    Ok(())
}

struct SGCSVWriter<'a> {
    mag_writer: csv::Writer<std::fs::File>,
    fft_writer: csv::Writer<std::fs::File>,
    timestamp_lut: &'a UnixTimestampLUT,
}

impl<'a> SGCSVWriter<'a> {
    pub fn new(
        save_folder: &PathBuf,
        sg_i: u8,
        timestamp_lut: &'a UnixTimestampLUT,
    ) -> Result<Self> {
        let mut mag_writer = csv::Writer::from_path(extend_path(
            save_folder,
            format!("sg-{}.mag.csv", sg_i).as_str(),
        ))?;
        mag_writer.write_record(&["boot timestamp", "unix timestamp", "reading"])?;
        let mut fft_writer = csv::Writer::from_path(extend_path(
            save_folder,
            format!("sg-{}.fft.csv", sg_i).as_str(),
        ))?;
        let mut title_row = Vec::<String>::new();
        title_row.push("boot timestamp".into());
        title_row.push("unix timestamp".into());
        for i in 0..200 {
            title_row.push(format!("{} Hz", i * 20));
        }
        fft_writer.write_record(title_row)?;

        Ok(Self {
            mag_writer,
            fft_writer,
            timestamp_lut,
        })
    }

    pub fn write(&mut self, reading: ProcessedSGReading) -> Result<()> {
        assert_eq!(reading.samples.len(), 80);
        assert_eq!(reading.amplitudes.len(), 400);

        // samples
        for i in 0..reading.samples.len() {
            let timestamp = reading.start_time + 5f64 * i as f64;
            let unix_timestamp = self.timestamp_lut.get_unix_timestamp(timestamp);
            let sample =
                half::f16::from_le_bytes([reading.samples[i * 2], reading.samples[i * 2 + 1]])
                    .to_f32();
            self.mag_writer.write_record([
                format!("{}", timestamp),
                unix_timestamp.map_or("".into(), |t| format!("{}", t)),
                format!("{}", sample),
            ])?;
        }

        // fft
        let unix_timestamp = self.timestamp_lut.get_unix_timestamp(reading.start_time);
        let mut row = Vec::<String>::new();
        row.push(format!("{}", reading.start_time));
        row.push(unix_timestamp.map_or("".into(), |t| format!("{}", t)));
        for i in 0..200 {
            let amplitude = half::f16::from_le_bytes([
                reading.amplitudes[i * 2],
                reading.amplitudes[i * 2 + 1],
            ])
            .to_f32();
            row.push(format!("{}", amplitude));
        }
        self.fft_writer.write_record(row)?;

        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.mag_writer.flush()?;
        self.fft_writer.flush()?;
        Ok(())
    }
}
