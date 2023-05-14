use super::timer::Timer;

pub trait PyroChannel {
    async fn wait_continuity_change(&mut self);
    async fn read_continuity(&mut self) -> bool;
    async fn set_enable(&mut self, enable: bool);
}

pub struct DummyPyroChannel<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyPyroChannel<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> PyroChannel for DummyPyroChannel<T> {
    async fn wait_continuity_change(&mut self) {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }

    async fn read_continuity(&mut self) -> bool {
        true
    }

    async fn set_enable(&mut self, _enable: bool) {}
}
