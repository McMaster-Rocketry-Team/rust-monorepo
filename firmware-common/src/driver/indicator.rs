use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

pub trait Indicator {
    async fn set_enable(&mut self, enable: bool);
}

impl<'a, M, T> Indicator for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Indicator,
{
    async fn set_enable(&mut self, enable: bool) {
        self.deref_mut().set_enable(enable).await
    }
}

pub struct DummyIndicator {}

impl Indicator for DummyIndicator {
    async fn set_enable(&mut self, _enable: bool) {}
}
