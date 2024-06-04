use core::{fmt::Debug, marker::PhantomData};

use embedded_hal_async::delay::DelayNs;
use ferraris_calibration::IMUReadingTrait;
use rkyv::{Archive, Deserialize, Serialize};

use crate::common::delta_factory::Deltable;

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

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct IMUReadingDelta<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: u8,
    pub acc: [u8; 3],
    pub gyro: [u8; 3],
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 10.0, f64, u8);
    fixed_point_factory!(Acc, -2.0, 2.0, f32, u8);
    fixed_point_factory!(Gyro, -2.0, 2.0, f32, u8);
}

impl<T:TimestampType> Deltable for IMUReading<T>{
    type DeltaType = IMUReadingDelta<T>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            _phantom: PhantomData,
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            acc: [
                self.acc[0] + factories::Acc::to_float(delta.acc[0]),
                self.acc[1] + factories::Acc::to_float(delta.acc[1]),
                self.acc[2] + factories::Acc::to_float(delta.acc[2]),
            ],
            gyro: [
                self.gyro[0] + factories::Gyro::to_float(delta.gyro[0]),
                self.gyro[1] + factories::Gyro::to_float(delta.gyro[1]),
                self.gyro[2] + factories::Gyro::to_float(delta.gyro[2]),
            ],
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(IMUReadingDelta {
            _phantom: PhantomData,
            timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
            acc: [
                factories::Acc::to_fixed_point(self.acc[0] - other.acc[0])?,
                factories::Acc::to_fixed_point(self.acc[1] - other.acc[1])?,
                factories::Acc::to_fixed_point(self.acc[2] - other.acc[2])?,
            ],
            gyro: [
                factories::Gyro::to_fixed_point(self.gyro[0] - other.gyro[0])?,
                factories::Gyro::to_fixed_point(self.gyro[1] - other.gyro[1])?,
                factories::Gyro::to_fixed_point(self.gyro[2] - other.gyro[2])?,
            ],
        })
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
