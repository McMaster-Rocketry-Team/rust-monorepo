pub trait PyroChannel {
    async fn wait_continuity_change(&mut self);
    async fn read_continuity(&mut self) -> bool;
    async fn set_enable(&mut self, enable: bool);
}
