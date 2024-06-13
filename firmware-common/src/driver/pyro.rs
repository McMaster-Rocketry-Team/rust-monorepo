use core::ops::DerefMut as _;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use embedded_hal_async::delay::DelayNs;

pub trait Continuity {
    type Error: defmt::Format + core::fmt::Debug;
    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error>;
    async fn read_continuity(&mut self) -> Result<bool, Self::Error>;
}

pub trait PyroCtrl {
    type Error: defmt::Format + core::fmt::Debug;
    async fn set_enable(&mut self, enable: bool) -> Result<(), Self::Error>;
}

pub struct DummyContinuity<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyContinuity<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Continuity for DummyContinuity<D> {
    type Error = ();

    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error> {
        loop {
            self.delay.delay_ms(1).await;
        }
    }

    async fn read_continuity(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct DummyPyroCtrl {}

impl PyroCtrl for DummyPyroCtrl {
    type Error = ();

    async fn set_enable(&mut self, _enable: bool) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, M, T> PyroCtrl for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: PyroCtrl,
{
    type Error = T::Error;

    async fn set_enable(&mut self, enable: bool) -> Result<(), Self::Error> {
        self.deref_mut().set_enable(enable).await
    }
}

impl<'a, M, T> Continuity for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Continuity,
{
    type Error = T::Error;

    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error> {
        self.deref_mut().wait_continuity_change().await
    }

    async fn read_continuity(&mut self) -> Result<bool, Self::Error> {
        self.deref_mut().read_continuity().await
    }
}
