pub trait Buzzer {
    async fn set_enable(&mut self, enable: bool);
    async fn set_frequency(&mut self, frequency: u32);
}

pub struct DummyBuzzer {}

impl Buzzer for DummyBuzzer {
    async fn set_enable(&mut self, _enable: bool) {}

    async fn set_frequency(&mut self, _frequency: u32) {}
}
