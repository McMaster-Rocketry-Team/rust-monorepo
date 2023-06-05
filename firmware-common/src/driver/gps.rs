use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use heapless::String;

use super::timer::Timer;

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

pub struct DummyGPS<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyGPS<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> GPS for DummyGPS<T> {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }
}


pub trait GPSCtrl {
    async fn reset(&mut self);

    async fn set_enable(&mut self, enable: bool);
}

pub struct DummyGPSCtrl {
}

impl GPSCtrl for DummyGPSCtrl {
    async fn reset(&mut self) {}

    async fn set_enable(&mut self, _enable: bool) {}
}