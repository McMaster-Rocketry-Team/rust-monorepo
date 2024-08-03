use std::{ops::Range, path::PathBuf};

use anyhow::Result;
use either::Either;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::{
        delta_logger::{delta_logger::UnixTimestampLog, prelude::DeltaLoggerReader},
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    driver::{serial::SplitableSerial, timestamp::BootTimestamp},
    RpcClient,
};
use map_range::MapRange;
use tokio::{fs::File, io::BufReader};

use crate::{pull_file::pull_file, reader::BufReaderWrapper, PullArgs};

pub async fn pull_delta_logs<D: SensorData, FF: F64FixedPointFactory>(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    save_folder: PathBuf,
    file_id: u64,
    file_type_name: &str,
    row_titles: Vec<String>,
    row_data_getter: impl Fn(D) -> Vec<String>,
) -> Result<()>
where
    [(); size_of::<D>() + 10]:,
{
    let mut vldl_path = save_folder.clone();
    // VLDL: void lake delta log
    vldl_path.push(format!("{}.{}.vldl", file_id, file_type_name));
    pull_file(
        rpc,
        PullArgs {
            file_id,
            host_path: vldl_path.clone(),
        },
    )
    .await?;

    let mut csv_path = save_folder.clone();
    csv_path.push(format!("{}.{}.csv", file_id, file_type_name));
    let reader = BufReader::new(File::open(vldl_path).await?);
    let reader = BufReaderWrapper(reader);
    let mut reader = DeltaLoggerReader::<D, _, FF>::new(reader);
    let mut csv_writer = csv::Writer::from_path(&csv_path)?;
    let mut title_row = Vec::<String>::new();
    title_row.push("boot timestamp".into());
    title_row.push("unix timestamp".into());
    title_row.extend(row_titles);
    csv_writer.write_record(&title_row)?;

    let mut last_unix_timestamp: Option<UnixTimestampLog> = None;
    let mut last_ranges: Option<(Range<f64>, Range<f64>)> = None;
    let mut readings_buffer: Vec<SensorReading<BootTimestamp, D>> = Vec::new();
    let mut write_buffer = |last_ranges: &mut Option<(Range<f64>, Range<f64>)>,
                            readings_buffer: &mut Vec<SensorReading<BootTimestamp, D>>|
     -> Result<(), anyhow::Error> {
        for reading in readings_buffer.iter() {
            let unix_timestamp =
                if let Some((boot_timestamp_range, unix_timestamp_range)) = last_ranges.clone() {
                    let unix_timestamp = reading
                        .timestamp
                        .map_range(boot_timestamp_range, unix_timestamp_range);
                    format!("{}", unix_timestamp)
                } else {
                    "".into()
                };

            let mut row = Vec::<String>::new();
            row.push(format!("{}", reading.timestamp));
            row.push(unix_timestamp);
            row.extend(row_data_getter(reading.data.clone()));
            csv_writer.write_record(&row)?;
        }
        readings_buffer.clear();
        Ok(())
    };
    while let Some(reading) = reader.read().await.unwrap() {
        match reading {
            Either::Left(reading) => {
                readings_buffer.push(reading);
            }
            Either::Right(new_unix_time_log) => {
                if let Some(last_unix_timestamp) = last_unix_timestamp.clone() {
                    let unix_timestamp_range =
                        last_unix_timestamp.unix_timestamp..new_unix_time_log.unix_timestamp;
                    let boot_timestamp_range =
                        last_unix_timestamp.boot_timestamp..new_unix_time_log.boot_timestamp;
                    last_ranges =
                        Some((unix_timestamp_range.clone(), boot_timestamp_range.clone()));
                }
                last_unix_timestamp = Some(new_unix_time_log);
                write_buffer(&mut last_ranges, &mut readings_buffer)?;
            }
        }
    }
    write_buffer(&mut last_ranges, &mut readings_buffer)?;
    csv_writer.flush()?;
    Ok(())
}
