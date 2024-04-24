use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use heapless::String;

use embedded_hal_async::delay::DelayNs;

pub struct NmeaSentence {
    pub sentence: String<84>,
    pub timestamp: f64,
}

pub trait GPS {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence;
}

impl<'a, M, T> GPS for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: GPS,
{
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        self.deref_mut().next_nmea_sentence().await
    }
}

pub struct DummyGPS<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyGPS<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> GPS for DummyGPS<D> {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        loop {
            self.delay.delay_ms(1_000).await;
        }
    }
}

pub trait GPSCtrl {
    async fn reset(&mut self);

    async fn set_enable(&mut self, enable: bool);
}

pub struct DummyGPSCtrl {}

impl GPSCtrl for DummyGPSCtrl {
    async fn reset(&mut self) {}

    async fn set_enable(&mut self, _enable: bool) {}
}
