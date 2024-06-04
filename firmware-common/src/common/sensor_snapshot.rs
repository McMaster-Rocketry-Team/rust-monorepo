use crate::driver::{barometer::BaroReading, imu::IMUReading, meg::MegReading, timestamp::BootTimestamp};
use rkyv::{Archive, Deserialize, Serialize};

use super::gps_parser::GPSLocation;

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub enum SensorReading {
    GPS(GPSLocation),
    IMU(IMUReading<BootTimestamp>),
    Baro(BaroReading<BootTimestamp>),
    Meg(MegReading),
    BatteryVoltage(BatteryVoltage),
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct BatteryVoltage {
    pub timestamp: f64,
    pub voltage: f32,
}

impl SensorReading {
    pub fn timestamp(&self) -> f64 {
        match self {
            SensorReading::GPS(gps) => gps.timestamp,
            SensorReading::IMU(imu) => imu.timestamp,
            SensorReading::Baro(baro) => baro.timestamp,
            SensorReading::Meg(meg) => meg.timestamp,
            SensorReading::BatteryVoltage(batt) => batt.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartialSensorSnapshot {
    pub timestamp: f64, // ms
    pub imu_reading: IMUReading<BootTimestamp>,
    pub baro_reading: Option<BaroReading<BootTimestamp>>,
}
