use core::fmt::Write;
use heapless::String;
use nalgebra::Vector3;

use super::timer::Timer;

pub struct IMUReading {
    pub timestamp: u64,    // ms
    pub acc: Vector3<f32>, // m/s^2
    pub gyro: Vector3<f32>,
}

impl defmt::Format for IMUReading {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "IMUReading {{ acc: {} {} {}, gyro: {} {} {} }}",
            self.acc.x,
            self.acc.y,
            self.acc.z,
            self.gyro.x,
            self.gyro.y,
            self.gyro.z,
        )
    }
}

pub trait IMU {
    type Error;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<IMUReading, Self::Error>;
}

pub struct DummyIMU<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyIMU<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> IMU for DummyIMU<T> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading, ()> {
        self.timer.sleep(1).await;
        Ok(IMUReading {
            timestamp: 0,
            acc: Vector3::new(0.0, 0.0, 0.0),
            gyro: Vector3::new(0.0, 0.0, 0.0),
        })
    }
}
