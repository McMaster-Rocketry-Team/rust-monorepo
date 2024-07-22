use core::marker::PhantomData;

use embedded_hal_async::delay::DelayNs;
use rkyv::{Archive, Deserialize, Serialize};
use core::future::Future;

use crate::{
    common::{delta_factory::Deltable, unix_clock::UnixClock}, fixed_point_factory2, fixed_point_factory_slope, Clock
};

use super::timestamp::{BootTimestamp, TimestampType, UnixTimestamp};

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

impl<U: UnitType> ADCReading<U, BootTimestamp> {
    pub fn to_unix_timestamp(
        self,
        unix_clock: UnixClock<impl Clock>,
    ) -> ADCReading<U, UnixTimestamp> {
        ADCReading {
            _phantom_unit: PhantomData,
            _phantom_timestamp: PhantomData,
            timestamp: unix_clock.convert_to_unix(self.timestamp),
            value: self.value,
        }
    }
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

fixed_point_factory2!(TimestampFac, f64, 450.0, 550.0, 1.0);
fixed_point_factory_slope!(ValueFac, 0.2, 500.0, 0.002);

#[derive(defmt::Format, Debug, Clone)]
pub struct ADCReadingDelta<U: UnitType, T: TimestampType> {
    _phantom_unit: PhantomData<U>,
    _phantom_timestamp: PhantomData<T>,
    #[defmt(Debug2Format)]
    pub timestamp: TimestampFacPacked,
    #[defmt(Debug2Format)]
    pub value: ValueFacPacked,
}

impl<U: UnitType, T: TimestampType> Deltable for ADCReading<U, T> {
    type DeltaType = ADCReadingDelta<U, T>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            _phantom_unit: PhantomData,
            _phantom_timestamp: PhantomData,
            timestamp: self.timestamp + TimestampFac::to_float(delta.timestamp),
            value: self.value + ValueFac::to_float(delta.value),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(ADCReadingDelta {
            _phantom_unit: PhantomData,
            _phantom_timestamp: PhantomData,
            timestamp: TimestampFac::to_fixed_point(self.timestamp - other.timestamp)?,
            value: ValueFac::to_fixed_point(self.value - other.value)?,
        })
    }
}

pub trait ADC<U: UnitType> {
    type Error: defmt::Format + core::fmt::Debug;

    fn read(&mut self) -> impl Future<Output = Result<ADCReading<U, BootTimestamp>, Self::Error>>;
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
