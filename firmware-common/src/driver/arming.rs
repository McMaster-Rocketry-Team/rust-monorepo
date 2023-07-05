use super::timer::Timer;

pub trait HardwareArming {
    type Error: defmt::Format;
    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error>;
    async fn read_arming(&mut self) -> Result<bool, Self::Error>;
}

pub struct DummyHardwareArming<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyHardwareArming<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> HardwareArming for DummyHardwareArming<T> {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error> {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }

    async fn read_arming(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
