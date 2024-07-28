use crate::common::delta_logger::bitslice_io::{
    BitArrayDeserializable, BitArraySerializable, BitSliceReader, BitSliceWriter,
};
use crate::common::delta_logger::bitvec_serialize_traits::FromBitSlice;
use crate::common::fixed_point::F32FixedPointFactory;
use crate::common::sensor_reading::{SensorData, SensorReading};
use crate::fixed_point_factory_slope;
use crate::common::delta_factory::Deltable;
use core::future::Future;
use core::{fmt::Debug, ops::DerefMut as _};
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use embedded_hal_async::delay::DelayNs;
use libm::powf;

use super::timestamp::BootTimestamp;

#[derive(defmt::Format, Debug, Clone)]
pub struct BaroData {
    pub temperature: f32, // C
    pub pressure: f32,    // Pa
}

impl BitArraySerializable for BaroData {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.temperature);
        writer.write(self.pressure);
    }
}

impl BitArrayDeserializable for BaroData {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            temperature: reader.read().unwrap(),
            pressure: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        32 + 32
    }
}

fixed_point_factory_slope!(TemperatureFac, 20.0, 5.0, 0.05);
fixed_point_factory_slope!(PressureFac, 4000.0, 5.0, 1.0);

#[derive(defmt::Format, Debug, Clone)]
pub struct BaroDataDelta {
    #[defmt(Debug2Format)]
    pub temperature: TemperatureFacPacked,
    #[defmt(Debug2Format)]
    pub pressure: PressureFacPacked,
}

impl BitArraySerializable for BaroDataDelta {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.temperature);
        writer.write(self.pressure);
    }
}

impl BitArrayDeserializable for BaroDataDelta {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            temperature: reader.read().unwrap(),
            pressure: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        TemperatureFacPacked::len_bits() + PressureFacPacked::len_bits()
    }
}

impl Deltable for BaroData {
    type DeltaType = BaroDataDelta;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            temperature: self.temperature + TemperatureFac::to_float(delta.temperature),
            pressure: self.pressure + PressureFac::to_float(delta.pressure),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(Self::DeltaType {
            temperature: TemperatureFac::to_fixed_point(self.temperature - other.temperature)?,
            pressure: PressureFac::to_fixed_point(self.pressure - other.pressure)?,
        })
    }
}

impl SensorData for BaroData {}

impl BaroData {
    pub fn altitude(&self) -> f32 {
        // see https://github.com/pimoroni/bmp280-python/blob/master/library/bmp280/__init__.py
        let air_pressure_hpa = self.pressure / 100.0;
        return ((powf(1013.25 / air_pressure_hpa, 1.0 / 5.257) - 1.0)
            * (self.temperature + 273.15))
            / 0.0065;
    }
}

pub trait Barometer {
    type Error: defmt::Format + Debug;

    fn reset(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
    fn read(
        &mut self,
    ) -> impl Future<Output = Result<SensorReading<BootTimestamp, BaroData>, Self::Error>>;
}

pub struct DummyBarometer<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyBarometer<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Barometer for DummyBarometer<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, BaroData>, ()> {
        self.delay.delay_ms(1).await;
        Ok(SensorReading::new(
            0.0,
            BaroData {
                temperature: 25.0,
                pressure: 101325.0,
            },
        ))
    }
}

impl<'a, M, T> Barometer for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Barometer,
{
    type Error = T::Error;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.deref_mut().reset().await
    }

    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, BaroData>, Self::Error> {
        self.deref_mut().read().await
    }
}
