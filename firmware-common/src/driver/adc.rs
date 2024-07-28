use core::marker::PhantomData;

use core::future::Future;
use embedded_hal_async::delay::DelayNs;

use crate::common::delta_logger::prelude::*;
use crate::common::fixed_point::F32FixedPointFactory;
use crate::common::sensor_reading::{SensorData, SensorReading};
use crate::fixed_point_factory_slope;

use super::timestamp::BootTimestamp;

pub trait UnitType: Clone + defmt::Format + core::fmt::Debug {}

#[derive(defmt::Format, Debug, Clone, PartialEq)]
pub struct Volt;

impl UnitType for Volt {}

#[derive(defmt::Format, Debug, Clone, PartialEq)]
pub struct Ampere;

impl UnitType for Ampere {}

#[derive(defmt::Format, Debug, Clone, PartialEq)]
pub struct ADCData<U: UnitType> {
    _phantom_unit: PhantomData<U>,
    pub value: f32,
}

impl<U: UnitType> ADCData<U> {
    pub fn new(value: f32) -> Self {
        Self {
            _phantom_unit: PhantomData,
            value,
        }
    }
}

impl<U: UnitType> BitArraySerializable for ADCData<U> {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.value);
    }
}

impl<U: UnitType> BitArrayDeserializable for ADCData<U> {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self::new(reader.read().unwrap())
    }

    fn len_bits() -> usize {
        32
    }
}

fixed_point_factory_slope!(ValueFac, 0.2, 500.0, 0.002);

#[derive(defmt::Format, Debug, Clone)]
pub struct ADCDataDelta<U: UnitType> {
    _phantom_unit: PhantomData<U>,
    #[defmt(Debug2Format)]
    pub value: ValueFacPacked,
}

impl<U: UnitType> BitArraySerializable for ADCDataDelta<U> {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.value);
    }
}

impl<U: UnitType> BitArrayDeserializable for ADCDataDelta<U> {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            _phantom_unit: PhantomData,
            value: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        ValueFacPacked::len_bits()
    }
}

impl<U: UnitType> Deltable for ADCData<U> {
    type DeltaType = ADCDataDelta<U>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            _phantom_unit: PhantomData,
            value: self.value + ValueFac::to_float(delta.value),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(ADCDataDelta {
            _phantom_unit: PhantomData,
            value: ValueFac::to_fixed_point(self.value - other.value)?,
        })
    }
}

impl<U: UnitType> SensorData for ADCData<U> {}

pub trait ADC<U: UnitType> {
    type Error: defmt::Format + core::fmt::Debug;

    fn read(
        &mut self,
    ) -> impl Future<Output = Result<SensorReading<BootTimestamp, ADCData<U>>, Self::Error>>;
}

pub struct DummyADC<D: DelayNs, U: UnitType> {
    _phantom_unit: PhantomData<U>,
    delay: D,
}

impl<D: DelayNs, U: UnitType> DummyADC<D, U> {
    pub fn new(delay: D) -> Self {
        Self {
            _phantom_unit: Default::default(),
            delay,
        }
    }
}

impl<D: DelayNs, U: UnitType> ADC<U> for DummyADC<D, U> {
    type Error = ();

    async fn read(&mut self) -> Result<SensorReading<BootTimestamp, ADCData<U>>, ()> {
        self.delay.delay_ms(1).await;
        Ok(SensorReading::new(0.0, ADCData::new(0.0)))
    }
}
