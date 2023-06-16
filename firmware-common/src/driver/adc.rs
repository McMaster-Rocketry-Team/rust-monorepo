use super::timer::Timer;

pub trait ADC {
    type Error:defmt::Format;

    async fn read(&mut self) -> Result<f32, Self::Error>;
}

pub struct DummyADC<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyADC<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> ADC for DummyADC<T> {
    type Error = ();

    async fn read(&mut self) -> Result<f32, ()> {
        self.timer.sleep(1.0).await;
        Ok(0.0)
    }
}
