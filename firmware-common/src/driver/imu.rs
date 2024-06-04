use core::{fmt::Debug, marker::PhantomData};

use embedded_hal_async::delay::DelayNs;
use ferraris_calibration::IMUReadingTrait;
use rkyv::{Archive, Deserialize, Serialize};

use super::timestamp::{BootTimestamp, TimestampType};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct IMUReading<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: f64, // ms
    pub acc: [f32; 3],  // m/s^2
    pub gyro: [f32; 3],
}

impl<T: TimestampType> IMUReadingTrait for IMUReading<T> {
    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    fn acc(&self) -> [f32; 3] {
        self.acc
    }

    fn gyro(&self) -> [f32; 3] {
        self.gyro
    }

    fn set_acc(&mut self, acc: [f32; 3]) {
        self.acc = acc;
    }

    fn set_gyro(&mut self, gyro: [f32; 3]) {
        self.gyro = gyro;
    }
}

pub trait IMU {
    type Error: defmt::Format + Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<IMUReading<BootTimestamp>, Self::Error>;
}

pub struct DummyIMU<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyIMU<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> IMU for DummyIMU<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading<BootTimestamp>, ()> {
        self.delay.delay_ms(1).await;
        Ok(IMUReading {
            _phantom: PhantomData,
            timestamp: 0.0,
            acc: [0.0, 0.0, 0.0],
            gyro: [0.0, 0.0, 0.0],
        })
    }
}
