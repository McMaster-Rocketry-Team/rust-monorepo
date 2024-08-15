use anyhow::Result;
use firmware_common::strain_gauges::ProcessedSGReading;
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use std::path::PathBuf;

use crate::common::extend_path;

pub struct SGCSVWriter {
    mag_writer: csv::Writer<std::fs::File>,
    fft_writer: csv::Writer<std::fs::File>,
}

impl SGCSVWriter {
    pub fn new(save_folder: &PathBuf, sg_i: u8) -> Result<Self> {
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
        })
    }

    pub fn write(
        &mut self,
        reading: ProcessedSGReading,
        unix_timestamp: Option<f64>,
    ) -> Result<()> {
        assert_eq!(reading.samples.len(), 80);
        assert_eq!(reading.amplitudes.len(), 400);

        // samples
        for i in 0..reading.samples.len() {
            let timestamp = reading.start_time + 5f64 * i as f64;
            let unix_timestamp = unix_timestamp.map(|t| t + 5f64 * i as f64);
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
