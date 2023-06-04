use super::timer::Timer;

pub trait Continuity {
    async fn wait_continuity_change(&mut self);
    async fn read_continuity(&mut self) -> bool;
}

pub trait PyroCtrl {
    async fn set_enable(&mut self, enable: bool);
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
    async fn wait_continuity_change(&mut self) {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }

    async fn read_continuity(&mut self) -> bool {
        true
    }
}

pub struct DummyPyroCtrl {}

impl PyroCtrl for DummyPyroCtrl {
    async fn set_enable(&mut self, _enable: bool) {}
}
