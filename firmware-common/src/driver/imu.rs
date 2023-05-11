use core::fmt::Write;
use heapless::String;
use nalgebra::Vector3;

use super::bus_error::BusError;

pub struct IMUReading {
    pub timestamp: u64,    // ms
    pub acc: Vector3<f32>, // m/s^2
    pub gyro: Vector3<f32>,
}

impl defmt::Format for IMUReading {
    fn format(&self, f: defmt::Formatter) {
        let mut message = String::<128>::new();
        core::write!(
            &mut message,
            "IMUReading {{ acc: {:.2} {:.2} {:.2}, gyro: {:.2} {:.2} {:.2} }}",
            self.acc.x,
            self.acc.y,
            self.acc.z,
            self.gyro.x,
            self.gyro.y,
            self.gyro.z,
        )
        .unwrap();
        defmt::write!(f, "{}", message.as_str())
    }
}

pub trait IMU {
    async fn reset(&mut self) -> Result<(), BusError>;
    async fn read(&mut self) -> Result<IMUReading, BusError>;
}
