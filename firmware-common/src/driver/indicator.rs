use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use futures::future::join;

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

pub struct MergedIndicator<A:Indicator, B:Indicator> {
    a: A,
    b: B,
}

impl<A:Indicator, B:Indicator> MergedIndicator<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A:Indicator, B:Indicator> Indicator for MergedIndicator<A, B> {
    async fn set_enable(&mut self, enable: bool) {
        join(self.a.set_enable(enable), self.b.set_enable(enable)).await;
    }
}

pub struct DummyIndicator {}

impl Indicator for DummyIndicator {
    async fn set_enable(&mut self, _enable: bool) {}
}
