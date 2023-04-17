pub trait Buzzer {
    async fn set_enable(&mut self, enable: bool);
    async fn set_frequency(&mut self, frequency: u32);
}