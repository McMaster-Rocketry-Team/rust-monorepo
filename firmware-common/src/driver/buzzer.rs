use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use embedded_hal_async::delay::DelayNs;

pub trait Buzzer {
    async fn play(&mut self, frequency: u32, duration_ms: u32);
}

impl<'a, M, T> Buzzer for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Buzzer,
{
    async fn play(&mut self, frequency: u32, duration_ms: u32) {
        self.deref_mut().play(frequency, duration_ms).await
    }
}

pub struct DummyBuzzer<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyBuzzer<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Buzzer for DummyBuzzer<D> {
    async fn play(&mut self, _frequency: u32, duration_ms: u32) {
        self.delay.delay_ms(duration_ms).await;
    }
}
