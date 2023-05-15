use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use heapless::String;

pub struct NmeaSentence {
    pub sentence: String<84>,
    pub timestamp: u64,
}

pub trait GPS {
    async fn reset(&mut self);

    async fn set_enable(&mut self, enable: bool);

    fn read_next_nmea_sentence(&mut self) -> Option<NmeaSentence>;
}

impl<'a, M, T> GPS for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: GPS,
{
    async fn reset(&mut self) {
        self.deref_mut().reset().await
    }

    async fn set_enable(&mut self, enable: bool) {
        self.deref_mut().set_enable(enable).await
    }

    fn read_next_nmea_sentence(&mut self) -> Option<NmeaSentence> {
        self.deref_mut().read_next_nmea_sentence()
    }
}

pub struct DummyGPS {}

impl GPS for DummyGPS {
    async fn reset(&mut self) {}

    async fn set_enable(&mut self, _enable: bool) {}

    fn read_next_nmea_sentence(&mut self) -> Option<NmeaSentence> {
        None
    }
}
