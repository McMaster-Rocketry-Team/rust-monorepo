use super::timer::Timer;

pub trait HardwareArming {
    async fn wait_arming_change(&mut self);
    async fn read_arming(&mut self) -> bool;
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
    async fn wait_arming_change(&mut self) {
        loop {
            self.timer.sleep(1000).await;
        }
    }

    async fn read_arming(&mut self) -> bool {
        true
    }
}
