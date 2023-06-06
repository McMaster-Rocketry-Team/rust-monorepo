use super::timer::Timer;

pub trait Continuity {
    type Error: defmt::Format;
    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error>;
    async fn read_continuity(&mut self) -> Result<bool, Self::Error>;
}

pub trait PyroCtrl {
    type Error: defmt::Format;
    async fn set_enable(&mut self, enable: bool) -> Result<(), Self::Error>;
}

pub struct DummyContinuity<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyContinuity<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> Continuity for DummyContinuity<T> {
    type Error = ();

    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error> {
        loop {
            self.timer.sleep(1000.0).await;
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
