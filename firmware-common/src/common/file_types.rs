use vlfs::FileType;

pub static DEVICE_CONFIG_FILE_TYPE: FileType = FileType(0);
// pub static BEACON_SENDER_LOG_FILE_TYPE: FileType = FileType(1);
pub static BENCHMARK_FILE_TYPE: FileType = FileType(2);
pub static CALIBRATION_FILE_TYPE: FileType = FileType(3);
pub static AVIONICS_SENSORS_FILE_TYPE: FileType = FileType(4);
pub static AVIONICS_LOG_FILE_TYPE: FileType = FileType(5);
pub static AVIONICS_UP_RIGHT_FILE_TYPE: FileType = FileType(6);
pub static GROUND_TEST_LOG_FILE_TYPE: FileType = FileType(7);
pub static FLIGHT_PROFILE_FILE_TYPE: FileType = FileType(8);
pub static AVIONICS_GPS_LOGGER_TIER_1: FileType = FileType(9);
pub static AVIONICS_GPS_LOGGER_TIER_2: FileType = FileType(10);
pub static AVIONICS_LOW_G_IMU_LOGGER_TIER_1: FileType = FileType(9);
pub static AVIONICS_LOW_G_IMU_LOGGER_TIER_2: FileType = FileType(10);
pub static AVIONICS_HIGH_G_IMU_LOGGER_TIER_1: FileType = FileType(11);
pub static AVIONICS_HIGH_G_IMU_LOGGER_TIER_2: FileType = FileType(12);
pub static AVIONICS_BARO_LOGGER_TIER_1: FileType = FileType(13);
pub static AVIONICS_BARO_LOGGER_TIER_2: FileType = FileType(14);
pub static AVIONICS_MAG_LOGGER_TIER_1: FileType = FileType(15);
pub static AVIONICS_MAG_LOGGER_TIER_2: FileType = FileType(16);
pub static AVIONICS_BATTERY_LOGGER_TIER_1: FileType = FileType(17);
pub static AVIONICS_BATTERY_LOGGER_TIER_2: FileType = FileType(18);
pub static UPRIGHT_VECTOR_AND_GYRO_OFFSET_FILE_TYPE: FileType = FileType(19);
pub static GROUND_TEST_BARO_FILE_TYPE: FileType = FileType(20);
pub static VACUUM_TEST_LOG_FILE_TYPE: FileType = FileType(21);
pub static VACUUM_TEST_BARO_LOGGER_TIER_1: FileType = FileType(22);
pub static VACUUM_TEST_BARO_LOGGER_TIER_2: FileType = FileType(23);
pub static SG_READINGS: FileType = FileType(24);
pub static SG_BATTERY_LOGGER: FileType = FileType(25);