mod list_files;
pub(crate) mod parse_serialized_enums;
mod probe_device_type;
pub(crate) mod pull_file;
pub(crate) mod parse_delta_readings;
pub(crate) mod readers;
mod sensor_reading_csv_writer;
pub(crate) mod unix_timestamp_lut;

use std::path::PathBuf;

pub use list_files::list_files;
pub use probe_device_type::probe_device_type;
pub use pull_file::pull_file;
pub use sensor_reading_csv_writer::SensorReadingCSVWriter;

pub fn extend_path(path: &PathBuf, extend: &str) -> PathBuf {
    let mut path = path.clone();
    path.push(extend);
    path
}
