pub trait CanBusMessage {
    fn id(&self) -> u32;
    fn rtr(&self) -> Option<usize>;
    fn data(&self) -> &[u8];
}

pub trait SplitableCanBus {
    type Error: defmt::Format + core::fmt::Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;

    async fn split(
        &mut self,
    ) -> (
        impl CanBusTX<Error = Self::Error>,
        impl CanBusRX<Error = Self::Error>,
    );
}

pub trait CanBusTX {
    type Error: defmt::Format + core::fmt::Debug;

    async fn send_data(&mut self, id: u32, data: &[u8]) -> Result<(), Self::Error>;
    async fn send_remote(&mut self, id: u32, length: usize) -> Result<(), Self::Error>;
}

pub trait CanBusRX {
    type Error: defmt::Format + core::fmt::Debug;

    async fn receive(&mut self) -> Result<impl CanBusMessage, Self::Error>;
}
