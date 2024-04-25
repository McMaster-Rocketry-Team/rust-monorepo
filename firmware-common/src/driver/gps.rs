use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use heapless::String;

pub use crate::common::gps_parser::GPSParser;
pub use crate::common::gps_parser::GPSLocation;
use embedded_hal_async::delay::DelayNs;
pub struct NmeaSentence {
    pub sentence: String<84>,
    pub timestamp: f64,
}

// FIXME error handling
pub trait GPS {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence;
    async fn reset(&mut self);
}

impl<'a, M, T> GPS for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: GPS,
{
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        self.deref_mut().next_nmea_sentence().await
    }

    async fn reset(&mut self) {
        self.deref_mut().reset().await;
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

    async fn reset(&mut self) {}
}

pub trait GPSPPS {
    async fn wait_for_pps(&mut self);
}

impl<'a, M, T> GPSPPS for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: GPSPPS,
{
    async fn wait_for_pps(&mut self) {
        self.deref_mut().wait_for_pps().await;
    }
}

pub struct DummyGPSPPS<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyGPSPPS<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> GPSPPS for DummyGPSPPS<D> {
    async fn wait_for_pps(&mut self) {
        loop {
            self.delay.delay_ms(1_000).await;
        }
    }
}
