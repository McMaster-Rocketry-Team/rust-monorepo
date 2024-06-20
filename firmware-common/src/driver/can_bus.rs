use heapless::Vec;

use crate::Delay;

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
    Data { id: u32, data: Vec<u8, 64> },
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

#[derive(Clone)]
pub struct DummyCanBus<D: Delay> {
    delay: D,
}

impl<D: Delay> DummyCanBus<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: Delay> SplitableCanBus for DummyCanBus<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn split(
        &mut self,
    ) -> (
        impl CanBusTX<Error = Self::Error>,
        impl CanBusRX<Error = Self::Error>,
    ) {
        (self.clone(), self.clone())
    }
}

impl<D: Delay> CanBusTX for DummyCanBus<D> {
    type Error = ();

    async fn send(&mut self, _frame: CanBusTXFrame) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<D: Delay> CanBusRX for DummyCanBus<D> {
    type Error = ();

    async fn receive(&mut self) -> Result<impl CanBusMessage, Self::Error> {
        loop {
            self.delay.delay_ms(1000).await;
        }
        Ok(DummyCanBusMessage)
    }
}

pub struct DummyCanBusMessage;

impl CanBusMessage for DummyCanBusMessage {
    fn id(&self) -> u32 {
        todo!()
    }

    fn rtr(&self) -> Option<usize> {
        todo!()
    }

    fn data(&self) -> &[u8] {
        todo!()
    }
}
