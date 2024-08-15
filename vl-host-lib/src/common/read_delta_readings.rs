use std::path::PathBuf;

use anyhow::Result;
use either::Either;
use firmware_common::{
    common::{
        delta_logger::delta_logger::DeltaLoggerReader,
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    driver::timestamp::BootTimestamp,
};
use tokio::{fs::File, io::BufReader};

use super::{readers::BufReaderWrapper, unix_timestamp_lut::UnixTimestampLUT};
use async_stream::stream;
use futures_core::stream::Stream;

pub async fn read_delta_readings<D: SensorData, FF: F64FixedPointFactory>(
    files: Vec<PathBuf>,
) -> Result<impl Stream<Item = (SensorReading<BootTimestamp, D>, Option<f64>)>>
where
    [(); size_of::<D>() + 10]:,
{
    // Pass 1: get all the unix timestamps
    let mut timestamp_lut = UnixTimestampLUT::new();
    for file_path in &files {
        let reader = BufReader::new(File::open(file_path).await?);
        let reader = BufReaderWrapper(reader);
        let mut reader = DeltaLoggerReader::<D, _, FF>::new(reader);

        while let Some(reading) = reader.read().await.unwrap() {
            if let Either::Right(unix_time_log) = reading {
                timestamp_lut
                    .add_timestamp(unix_time_log.boot_timestamp, unix_time_log.unix_timestamp)
            }
        }
    }
    // sort just in case
    timestamp_lut.sort_timestamps();

    // Pass 2: read all the readings and convert timestamps
    let stream = stream! {
        for file_path in &files {
            let reader = BufReader::new(File::open(file_path).await.unwrap());
            let reader = BufReaderWrapper(reader);
            let mut reader = DeltaLoggerReader::<D, _, FF>::new(reader);
            while let Some(reading) = reader.read().await.unwrap() {
                if let Either::Left(reading) = reading {
                    let unix_timestamp = timestamp_lut.get_unix_timestamp(reading.timestamp);

                    yield (reading.clone(), unix_timestamp);
                }
            }
        }
    };
    Ok(stream)
}
