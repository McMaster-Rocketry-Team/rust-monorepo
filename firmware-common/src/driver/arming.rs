pub trait HardwareArming {
    async fn wait_arming_change(&mut self);
    async fn read_arming(&mut self) -> bool;
}
