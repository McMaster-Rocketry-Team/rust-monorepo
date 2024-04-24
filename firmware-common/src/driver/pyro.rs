use embedded_hal_async::delay::DelayNs;

pub trait Continuity {
    type Error: defmt::Format;
    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error>;
    async fn read_continuity(&mut self) -> Result<bool, Self::Error>;
}

pub trait PyroCtrl {
    type Error: defmt::Format;
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
