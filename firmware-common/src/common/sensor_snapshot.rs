use ferraris_calibration::IMUReading;

use crate::driver::barometer::BaroReading;

use super::gps_parser::GPSLocation;

#[derive(Debug, Clone)]
pub struct SensorSnapshot {
    pub timestamp: f64, // ms
    pub gps_location: GPSLocation,
    pub imu_reading: IMUReading,
    pub baro_reading: BaroReading,
    pub pyro1_continuity: bool,
    pub pyro2_continuity: bool,
    pub pyro3_continuity: bool,
}