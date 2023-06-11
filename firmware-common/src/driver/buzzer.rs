use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

use super::timer::Timer;

pub trait Buzzer {
    async fn play(&mut self, frequency: u32, duration_ms:f64);
}

impl<'a, M, T> Buzzer for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Buzzer,
{
    async fn play(&mut self, frequency: u32, duration_ms:f64) {
        self.deref_mut().play(frequency,duration_ms).await
    }
}

pub struct DummyBuzzer<T:Timer> {
    timer: T,
}

impl<T:Timer> DummyBuzzer<T> {
    pub fn new(timer: T) -> Self {
        Self {timer}
    }
}

impl<T:Timer> Buzzer for DummyBuzzer<T> {
    async fn play(&mut self, frequency: u32, duration_ms:f64) {
        self.timer.sleep(duration_ms).await;
    }
}
