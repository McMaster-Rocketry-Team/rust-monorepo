use embedded_hal_async::delay::DelayNs;

pub trait HardwareArming {
    type Error: defmt::Format;
    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error>;
    async fn read_arming(&mut self) -> Result<bool, Self::Error>;
}

pub struct DummyHardwareArming<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyHardwareArming<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> HardwareArming for DummyHardwareArming<D> {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error> {
        loop {
            self.delay.delay_ms(1000).await;
        }
    }

    async fn read_arming(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
