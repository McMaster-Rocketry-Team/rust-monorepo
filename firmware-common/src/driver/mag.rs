use core::fmt::Debug;

use embedded_hal_async::delay::DelayNs;

use super::timestamp::BootTimestamp;
use crate::common::delta_logger::prelude::*;
use crate::{
    common::{
        delta_logger::delta_factory::Deltable,
        fixed_point::F32FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    fixed_point_factory_slope,
};

#[derive(defmt::Format, Debug, Clone)]
pub struct MagData {
    pub mag: [f32; 3], // gauss
}

impl BitArraySerializable for MagData {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.mag);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            mag: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        <[f32; 3]>::len_bits()
    }
}

fixed_point_factory_slope!(MagFac, 0.5, 5.0, 0.0001);

#[derive(defmt::Format, Debug, Clone)]
pub struct MagDataDelta {
    #[defmt(Debug2Format)]
    pub mag: [MagFacPacked; 3],
}

impl BitArraySerializable for MagDataDelta {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.mag);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            mag: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        <[MagFacPacked; 3]>::len_bits()
    }
}

impl Deltable for MagData {
    type DeltaType = MagDataDelta;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            mag: [
                self.mag[0] + MagFac::to_float(delta.mag[0]),
                self.mag[1] + MagFac::to_float(delta.mag[1]),
                self.mag[2] + MagFac::to_float(delta.mag[2]),
            ],
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(Self::DeltaType {
            mag: [
                MagFac::to_fixed_point(self.mag[0] - other.mag[0])?,
                MagFac::to_fixed_point(self.mag[1] - other.mag[1])?,
                MagFac::to_fixed_point(self.mag[2] - other.mag[2])?,
            ],
        })
    }
}

impl SensorData for MagData {}

pub trait Magnetometer {
    type Error: defmt::Format + Debug;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, MagData>, Self::Error>;
}

pub struct DummyMagnetometer<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyMagnetometer<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Magnetometer for DummyMagnetometer<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, MagData>, ()> {
        self.delay.delay_ms(1).await;
        Ok(SensorReading::new(
            0.0,
            MagData {
                mag: [0.0, 0.0, 0.0],
            },
        ))
    }
}
