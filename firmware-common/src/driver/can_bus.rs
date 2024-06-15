pub trait CanBusTX {
    type Error: defmt::Format + core::fmt::Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;

    async fn send(&mut self, raw_id: u32, data: &[u8]) -> Result<(), Self::Error>;
}
