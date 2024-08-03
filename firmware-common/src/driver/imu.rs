use core::fmt::Debug;

use embedded_hal_async::delay::DelayNs;
use ferraris_calibration::IMUReadingTrait;

use crate::{
    common::{
        delta_logger::{delta_factory::Deltable, prelude::*},
        fixed_point::F32FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    fixed_point_factory_slope,
};

use super::timestamp::{BootTimestamp, TimestampType};

#[derive(defmt::Format, Debug, Clone)]
pub struct IMUData {
    pub acc: [f32; 3],  // m/s^2
    pub gyro: [f32; 3], // deg/s
}

impl BitArraySerializable for IMUData {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.acc);
        writer.write(self.gyro);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            acc: reader.read().unwrap(),
            gyro: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        <[f32; 3]>::len_bits() + <[f32; 3]>::len_bits()
    }
}

fixed_point_factory_slope!(AccFac, 100.0, 5.0, 0.01);
fixed_point_factory_slope!(GyroFac, 100.0, 5.0, 0.01);

#[derive(defmt::Format, Debug, Clone)]
pub struct IMUDataDelta {
    #[defmt(Debug2Format)]
    pub acc: [AccFacPacked; 3],
    #[defmt(Debug2Format)]
    pub gyro: [GyroFacPacked; 3],
}

impl BitArraySerializable for IMUDataDelta {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.acc);
        writer.write(self.gyro);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            acc: reader.read().unwrap(),
            gyro: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        <[AccFacPacked; 3]>::len_bits() + <[GyroFacPacked; 3]>::len_bits()
    }
}

impl Deltable for IMUData {
    type DeltaType = IMUDataDelta;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            acc: [
                self.acc[0] + AccFac::to_float(delta.acc[0]),
                self.acc[1] + AccFac::to_float(delta.acc[1]),
                self.acc[2] + AccFac::to_float(delta.acc[2]),
            ],
            gyro: [
                self.gyro[0] + GyroFac::to_float(delta.gyro[0]),
                self.gyro[1] + GyroFac::to_float(delta.gyro[1]),
                self.gyro[2] + GyroFac::to_float(delta.gyro[2]),
            ],
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(IMUDataDelta {
            acc: [
                AccFac::to_fixed_point(self.acc[0] - other.acc[0])?,
                AccFac::to_fixed_point(self.acc[1] - other.acc[1])?,
                AccFac::to_fixed_point(self.acc[2] - other.acc[2])?,
            ],
            gyro: [
                GyroFac::to_fixed_point(self.gyro[0] - other.gyro[0])?,
                GyroFac::to_fixed_point(self.gyro[1] - other.gyro[1])?,
                GyroFac::to_fixed_point(self.gyro[2] - other.gyro[2])?,
            ],
        })
    }
}

impl SensorData for IMUData {}

impl<T: TimestampType> IMUReadingTrait for SensorReading<T, IMUData> {
    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    fn acc(&self) -> [f32; 3] {
        self.data.acc
    }

    fn gyro(&self) -> [f32; 3] {
        self.data.gyro
    }

    fn set_acc(&mut self, acc: [f32; 3]) {
        self.data.acc = acc;
    }

    fn set_gyro(&mut self, gyro: [f32; 3]) {
        self.data.gyro = gyro;
    }
}

pub trait IMU {
    type Error: defmt::Format + Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, IMUData>, Self::Error>;
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

    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, IMUData>, ()> {
        self.delay.delay_ms(1).await;
        Ok(SensorReading::new(
            0.0,
            IMUData {
                acc: [0.0, 0.0, 0.0],
                gyro: [0.0, 0.0, 0.0],
            },
        ))
    }
}
