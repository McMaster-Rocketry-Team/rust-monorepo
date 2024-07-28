use crate::driver::{barometer::BaroData, gps::GPSData, imu::IMUData, mag::MagData, timestamp::BootTimestamp};
use rkyv::{Archive, Deserialize, Serialize};

use super::sensor_reading::SensorReading;


#[derive(Debug, Clone)]
pub enum SensorReadingEnum {
    GPS(SensorReading<BootTimestamp, GPSData>),
    IMU(SensorReading<BootTimestamp, IMUData>),
    Baro(SensorReading<BootTimestamp, BaroData>),
    Mag(SensorReading<BootTimestamp, MagData>),
    BatteryVoltage(BatteryVoltage),
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct BatteryVoltage {
    pub timestamp: f64,
    pub voltage: f32,
}

impl SensorReadingEnum {
    pub fn timestamp(&self) -> f64 {
        match self {
            SensorReadingEnum::GPS(gps) => gps.timestamp,
            SensorReadingEnum::IMU(imu) => imu.timestamp,
            SensorReadingEnum::Baro(baro) => baro.timestamp,
            SensorReadingEnum::Mag(mag) => mag.timestamp,
            SensorReadingEnum::BatteryVoltage(batt) => batt.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartialSensorSnapshot {
    pub timestamp: f64, // ms
    pub imu_reading: SensorReading<BootTimestamp, IMUData>,
    pub baro_reading: Option<SensorReading<BootTimestamp, BaroData>>,
}
