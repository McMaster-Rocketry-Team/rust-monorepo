pub trait Indicator {
    async fn set_enable(&mut self, enable: bool);
}

pub struct DummyIndicator {}

impl Indicator for DummyIndicator {
    async fn set_enable(&mut self, _enable: bool) {}
}
