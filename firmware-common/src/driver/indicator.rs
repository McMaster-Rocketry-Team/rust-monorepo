pub trait Indicator {
    async fn set_enable(&mut self, enable: bool);
}