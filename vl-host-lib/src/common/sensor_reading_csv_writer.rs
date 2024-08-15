use anyhow::Result;
use firmware_common::{
    common::sensor_reading::{SensorData, SensorReading},
    driver::timestamp::BootTimestamp,
};
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use std::{fs::File, marker::PhantomData, path::PathBuf};

pub struct SensorReadingCSVWriter<D: SensorData, G: Fn(D) -> Vec<String>> {
    writer: csv::Writer<File>,
    row_data_getter: G,
    phantom: PhantomData<D>,
}

impl<D: SensorData, G: Fn(D) -> Vec<String>> SensorReadingCSVWriter<D, G> {
    pub fn new(file_path: &PathBuf, row_titles: &[&str], row_data_getter: G) -> Result<Self> {
        let mut writer = csv::Writer::from_path(file_path)?;
        let mut title_row = Vec::<&str>::new();
        title_row.push("boot timestamp");
        title_row.push("unix timestamp");
        title_row.extend(row_titles);
        writer.write_record(&title_row)?;

        Ok(Self {
            writer,
            row_data_getter,
            phantom: PhantomData,
        })
    }

    pub fn write(
        &mut self,
        reading: SensorReading<BootTimestamp, D>,
        unix_timestamp: Option<f64>,
    ) -> Result<()> {
        let mut row = Vec::<String>::new();

        row.push(format!("{}", reading.timestamp));
        row.push(unix_timestamp.map_or("".into(), |t| format!("{}", t)));
        row.extend((self.row_data_getter)(reading.data));
        self.writer.write_record(row)?;
        Ok(())
    }

    pub async fn write_all(
        &mut self,
        stream: impl Stream<Item = (SensorReading<BootTimestamp, D>, Option<f64>)>,
    ) -> Result<()> {
        pin_mut!(stream);
        while let Some((reading, unix_timestamp)) = stream.next().await {
            self.write(reading, unix_timestamp)?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
