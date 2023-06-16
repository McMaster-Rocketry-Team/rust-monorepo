use crate::driver::{barometer::BaroReading, meg::MegReading};
use ferraris_calibration::IMUReading;
use rkyv::{Archive, Deserialize, Serialize};

use super::gps_parser::GPSLocation;

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub enum SensorReading {
    GPS(GPSLocation),
    IMU(IMUReading),
    Baro(BaroReading),
    Meg(MegReading),
    BatteryVoltage { timestamp: f64, voltage: f32 },
}

impl SensorReading {
    pub fn timestamp(&self) -> f64 {
        match self {
            SensorReading::GPS(gps) => gps.timestamp,
            SensorReading::IMU(imu) => imu.timestamp,
            SensorReading::Baro(baro) => baro.timestamp,
            SensorReading::Meg(meg) => meg.timestamp,
            SensorReading::BatteryVoltage { timestamp, .. } => *timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartialSensorSnapshot {
    pub timestamp: f64, // ms
    pub imu_reading: IMUReading,
    pub baro_reading: Option<BaroReading>,
}
