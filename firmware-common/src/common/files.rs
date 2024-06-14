use vlfs::FileType;

pub static DEVICE_CONFIG_FILE_TYPE: FileType = FileType(0);
pub static BEACON_SENDER_LOG_FILE_TYPE: FileType = FileType(1);
pub static BENCHMARK_FILE_TYPE: FileType = FileType(2);
pub static CALIBRATION_FILE_TYPE: FileType = FileType(3);
pub static AVIONICS_SENSORS_FILE_TYPE: FileType = FileType(4);
pub static AVIONICS_LOG_FILE_TYPE: FileType = FileType(5);
pub static AVIONICS_UP_RIGHT_FILE_TYPE: FileType = FileType(6);
pub static GROUND_TEST_LOG_FILE_TYPE: FileType = FileType(7);