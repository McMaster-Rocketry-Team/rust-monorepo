use super::serial::SplitableSerial;

pub trait SplitableUSB: SplitableSerial {
    async fn wait_connection(&mut self);
}
