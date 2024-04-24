use embedded_hal_async::delay::DelayNs;

pub trait ADC {
    type Error: defmt::Format;

    async fn read(&mut self) -> Result<f32, Self::Error>;
}

pub struct DummyADC<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyADC<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> ADC for DummyADC<D> {
    type Error = ();

    async fn read(&mut self) -> Result<f32, ()> {
        self.delay.delay_ms(1).await;
        Ok(0.0)
    }
}
