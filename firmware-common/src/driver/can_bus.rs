use heapless::Vec;

use crate::common::can_bus::message::CanBusMessage;

use super::delay::Delay;

pub trait CanBusRawMessage {
    fn id(&self) -> u32;
    fn rtr(&self) -> bool;
    fn data(&self) -> &[u8];
}

pub trait SplitableCanBus {
    type Error: defmt::Format + core::fmt::Debug;
    type TX: CanBusTX<Error = Self::Error>;
    type RX: CanBusRX<Error = Self::Error>;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    fn configure_self_node(&mut self, node_type: u8, node_id: u16);

    // TODO split with &self?
    fn split(self) -> (Self::TX, Self::RX);
}

pub enum CanBusTXFrame {
    Data { id: u32, data: Vec<u8, 64> },
    Remote { id: u32, length: usize },
}

pub trait CanBusTX {
    type Error: defmt::Format + core::fmt::Debug;

    /// priority can be 0 - 7, 7 being the highest priority
    async fn send<T: CanBusMessage>(
        &mut self,
        message: &T,
        priority: u8,
    ) -> Result<(), Self::Error>;

    /// priority can be 0 - 7, 7 being the highest priority
    async fn send_remote<T: CanBusMessage>(&mut self, priority: u8) -> Result<(), Self::Error>;
}

pub trait CanBusRX {
    type Error: defmt::Format + core::fmt::Debug;
    type Message: CanBusRawMessage;

    async fn receive(&mut self) -> Result<Self::Message, Self::Error>;
}

pub fn can_node_id_from_serial_number(serial_number: &[u8]) -> u16 {
    let crc = crc::Crc::<u16>::new(&crc::CRC_16_GSM);
    crc.checksum(serial_number) & 0xFFF
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
    type TX = Self;
    type RX = Self;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn configure_self_node(&mut self, _node_type: u8, _node_id: u16) {}

    fn split(self) -> (Self::TX, Self::RX) {
        (self.clone(), self.clone())
    }
}

impl<D: Delay> CanBusTX for DummyCanBus<D> {
    type Error = ();

    async fn send<T: CanBusMessage>(
        &mut self,
        _message: &T,
        _priority: u8,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn send_remote<T: CanBusMessage>(&mut self, _priority: u8) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<D: Delay> CanBusRX for DummyCanBus<D> {
    type Error = ();
    type Message = DummyCanBusMessage;

    async fn receive(&mut self) -> Result<Self::Message, Self::Error> {
        loop {
            self.delay.delay_ms(1000.0).await;
        }
    }
}

pub struct DummyCanBusMessage;

impl CanBusRawMessage for DummyCanBusMessage {
    fn id(&self) -> u32 {
        todo!()
    }

    fn rtr(&self) -> bool {
        todo!()
    }

    fn data(&self) -> &[u8] {
        todo!()
    }
}
