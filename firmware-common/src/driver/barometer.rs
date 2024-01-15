use super::timer::Timer;
use core::fmt::Debug;
use libm::powf;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, Default, Archive, Deserialize, Serialize)]
pub struct BaroReading {
    pub timestamp: f64,   // ms
    pub temperature: f32, // C
    pub pressure: f32,    // Pa
}

impl BaroReading {
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
    async fn read(&mut self) -> Result<BaroReading, Self::Error>;
}

pub struct DummyBarometer<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyBarometer<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}
impl<T: Timer> Barometer for DummyBarometer<T> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<BaroReading, ()> {
        self.timer.sleep(1.0).await;
        Ok(BaroReading {
            timestamp: 0.0,
            temperature: 0.0,
            pressure: 0.0,
        })
    }
}
