use heapless::Vec;

pub trait CanBusMessage {
    fn id(&self) -> u32;
    fn rtr(&self) -> Option<usize>;
    fn data(&self) -> &[u8];
}

pub trait SplitableCanBus {
    type Error: defmt::Format + core::fmt::Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;

    fn split(
        &mut self,
    ) -> (
        impl CanBusTX<Error = Self::Error>,
        impl CanBusRX<Error = Self::Error>,
    );
}

pub enum CanBusTXFrame {
    Data { id: u32, data: Vec<u8,64> },
    Remote { id: u32, length: usize },
}

pub trait CanBusTX {
    type Error: defmt::Format + core::fmt::Debug;

    async fn send(&mut self, frame: CanBusTXFrame) -> Result<(), Self::Error>;
}

pub trait CanBusRX {
    type Error: defmt::Format + core::fmt::Debug;

    async fn receive(&mut self) -> Result<impl CanBusMessage, Self::Error>;
}
