use ferraris_calibration::IMUReading;

use crate::driver::barometer::BaroReading;

use super::gps_parser::GPSLocation;

#[derive(Debug, Clone)]
pub struct SensorSnapshot {
    pub timestamp: f64, // ms
    pub gps_location: GPSLocation,
    pub imu_reading: IMUReading,
    pub baro_reading: BaroReading,
}

#[derive(Debug, Clone)]
pub struct PartialSensorSnapshot {
    pub timestamp: f64, // ms
    pub imu_reading: IMUReading,
    pub gps_location: Option<GPSLocation>,
    pub baro_reading: Option<BaroReading>,
}
