use core::marker::PhantomData;

use embedded_hal_async::delay::DelayNs;
use rkyv::{Archive, Deserialize, Serialize};

use crate::common::delta_factory::Deltable;

use super::timestamp::{BootTimestamp, TimestampType};

pub trait UnitType: Clone {}

#[derive(defmt::Format, Debug, Clone)]
pub struct Volt;

impl UnitType for Volt {}

#[derive(defmt::Format, Debug, Clone)]
pub struct Ampere;

impl UnitType for Ampere {}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ADCReading<U: UnitType, T: TimestampType> {
    _phantom_unit: PhantomData<U>,
    _phantom_timestamp: PhantomData<T>,
    pub timestamp: f64,
    pub value: f32,
}

impl<U: UnitType, T: TimestampType> ADCReading<U, T> {
    pub fn new(timestamp: f64, value: f32) -> Self {
        Self {
            _phantom_unit: PhantomData,
            _phantom_timestamp: PhantomData,
            timestamp,
            value,
        }
    }
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ADCReadingDelta<U: UnitType, T: TimestampType> {
    _phantom_unit: PhantomData<U>,
    _phantom_timestamp: PhantomData<T>,
    pub timestamp: u16,
    pub value: u16,
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 1200.0, f64, u16);
    fixed_point_factory!(Value, -200.0, 200.0, f32, u16);
}

impl<U: UnitType, T: TimestampType> Deltable for ADCReading<U, T> {
    type DeltaType = ADCReadingDelta<U, T>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self{
            _phantom_unit: PhantomData,
            _phantom_timestamp: PhantomData,
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            value: self.value + factories::Value::to_float(delta.value),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(
            ADCReadingDelta {
                _phantom_unit: PhantomData,
                _phantom_timestamp: PhantomData,
                timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
                value: factories::Value::to_fixed_point(self.value - other.value)?,
            }
        )
    }
}

pub trait ADC<U: UnitType> {
    type Error: defmt::Format + core::fmt::Debug;

    async fn read(&mut self) -> Result<ADCReading<U, BootTimestamp>, Self::Error>;
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

    async fn read(&mut self) -> Result<ADCReading<U, BootTimestamp>, ()> {
        self.delay.delay_ms(1).await;
        Ok(ADCReading::new(0.0, 0.0))
    }
}
