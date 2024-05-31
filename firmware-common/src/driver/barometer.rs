use core::{fmt::Debug, marker::PhantomData};
use embedded_hal_async::delay::DelayNs;
use libm::powf;
use rkyv::{Archive, Deserialize, Serialize};

use super::timestamp::{BootTimestamp, TimestampType};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct BaroReading<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: f64,   // ms
    pub temperature: f32, // C
    pub pressure: f32,    // Pa
}

impl<T: TimestampType> BaroReading<T> {
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

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<BaroReading<BootTimestamp>, Self::Error>;
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

    async fn read(&mut self) -> Result<BaroReading<BootTimestamp>, ()> {
        self.delay.delay_ms(1).await;
        Ok(BaroReading {
            _phantom: PhantomData,
            timestamp: 0.0,
            temperature: 25.0,
            pressure: 101325.0,
        })
    }
}
