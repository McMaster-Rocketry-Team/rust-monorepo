use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

pub trait Buzzer {
    async fn set_enable(&mut self, enable: bool);
    async fn set_frequency(&mut self, frequency: u32);
}

impl<'a, M, T> Buzzer for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Buzzer,
{
    async fn set_enable(&mut self, enable: bool) {
        self.deref_mut().set_enable(enable).await
    }

    async fn set_frequency(&mut self, frequency: u32) {
        self.deref_mut().set_frequency(frequency).await
    }
}

pub struct DummyBuzzer {}

impl Buzzer for DummyBuzzer {
    async fn set_enable(&mut self, _enable: bool) {}

    async fn set_frequency(&mut self, _frequency: u32) {}
}
