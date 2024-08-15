use async_stream::stream;
use firmware_common::strain_gauges::{
    mid_prio::{SGReadingLog, SGReadingLoggerReader},
    ProcessedSGReading,
};
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use std::path::PathBuf;
use tokio::fs::File;

use anyhow::Result;

use crate::common::{
    parse_serialized_enums::parse_serialized_enums, readers::BufReaderWrapper,
    unix_timestamp_lut::UnixTimestampLUT,
};

pub async fn parse_sg_data(
    file_path: PathBuf,
) -> Result<impl Stream<Item = (ProcessedSGReading, Option<f64>)>> {
    // Pass 1: get all the unix timestamps
    let mut timestamp_lut = UnixTimestampLUT::new();
    let stream =
        parse_serialized_enums::<SGReadingLoggerReader<BufReaderWrapper<File>>>(file_path.clone())
            .await?;
    pin_mut!(stream);

    while let Some(reading) = stream.next().await {
        if let SGReadingLog::UnixTimestampLog(log) = reading {
            timestamp_lut.add_timestamp(log.boot_timestamp, log.unix_timestamp)
        }
    }
    timestamp_lut.sort_timestamps();

    // Pass 2
    let stream = stream! {
        let enum_stream = parse_serialized_enums::<SGReadingLoggerReader<BufReaderWrapper<File>>>(
            file_path.clone(),
        )
        .await.unwrap();
        pin_mut!(enum_stream);
        while let Some(reading) = enum_stream.next().await {
            if let SGReadingLog::ProcessedSGReading(reading) = reading {
                let unix_timestamp = timestamp_lut.get_unix_timestamp(reading.start_time);
                yield (reading, unix_timestamp);
            }
        }
    };

    Ok(stream)
}
