use core::cell::RefCell;
use core::convert::Infallible;
use core::marker::PhantomData;

use crate::common::can_bus::message::CanBusMessage;

use super::delay::Delay;

pub trait CanBusRawMessage {
    fn timestamp(&self) -> f64;
    fn id(&self) -> u32;
    fn rtr(&self) -> bool;
    fn data(&self) -> &[u8];
}

pub trait SplitableCanBus {
    type Error: defmt::Format + core::fmt::Debug;
    type TX<'a>: CanBusTX<Error = Self::Error>
    where
        Self: 'a;
    type RX<'a>: CanBusRX<Error = Self::Error>
    where
        Self: 'a;

    fn split(&mut self) -> (Self::TX<'_>, Self::RX<'_>);
}

pub trait CanBusTX {
    type Error: defmt::Format + core::fmt::Debug;

    fn configure_self_node(&mut self, node_type: u8, node_id: u16);

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

pub struct SplitableCanBusWrapper<
    T: CanBusTX<Error = E>,
    R: CanBusRX<Error = E>,
    E: defmt::Format + core::fmt::Debug,
> {
    _phantom_data: PhantomData<E>,
    tx: RefCell<T>,
    rx: RefCell<R>,
}

impl<T: CanBusTX<Error = E>, R: CanBusRX<Error = E>, E: defmt::Format + core::fmt::Debug>
    SplitableCanBusWrapper<T, R, E>
{
    pub fn new(tx: T, rx: R) -> Self {
        Self {
            _phantom_data: PhantomData,
            tx: RefCell::new(tx),
            rx: RefCell::new(rx),
        }
    }
}

impl<T: CanBusTX<Error = E>, R: CanBusRX<Error = E>, E: defmt::Format + core::fmt::Debug>
    SplitableCanBus for SplitableCanBusWrapper<T, R, E>
{
    type Error = E;

    type TX<'a> = TXGuard<'a, T, R, E>
    where
        Self: 'a;

    type RX<'a> = RXGuard<'a, T, R, E>
    where
        Self: 'a;

    fn split(&mut self) -> (Self::TX<'_>, Self::RX<'_>) {
        (TXGuard { wrapper: self }, RXGuard { wrapper: self })
    }
}

pub struct TXGuard<
    'a,
    T: CanBusTX<Error = E>,
    R: CanBusRX<Error = E>,
    E: defmt::Format + core::fmt::Debug,
> {
    wrapper: &'a SplitableCanBusWrapper<T, R, E>,
}

impl<'a, T: CanBusTX<Error = E>, R: CanBusRX<Error = E>, E: defmt::Format + core::fmt::Debug>
    CanBusTX for TXGuard<'a, T, R, E>
{
    type Error = E;

    fn configure_self_node(&mut self, node_type: u8, node_id: u16) {
        let mut tx = self.wrapper.tx.borrow_mut();
        tx.configure_self_node(node_type, node_id);
    }

    async fn send<M: CanBusMessage>(
        &mut self,
        message: &M,
        priority: u8,
    ) -> Result<(), Self::Error> {
        let mut tx = self.wrapper.tx.borrow_mut();
        tx.send(message, priority).await
    }

    async fn send_remote<M: CanBusMessage>(&mut self, priority: u8) -> Result<(), Self::Error> {
        let mut tx = self.wrapper.tx.borrow_mut();
        tx.send_remote::<M>(priority).await
    }
}

pub struct RXGuard<
    'a,
    T: CanBusTX<Error = E>,
    R: CanBusRX<Error = E>,
    E: defmt::Format + core::fmt::Debug,
> {
    wrapper: &'a SplitableCanBusWrapper<T, R, E>,
}

impl<'a, T: CanBusTX<Error = E>, R: CanBusRX<Error = E>, E: defmt::Format + core::fmt::Debug>
    CanBusRX for RXGuard<'a, T, R, E>
{
    type Error = E;
    type Message = R::Message;

    async fn receive(&mut self) -> Result<Self::Message, Self::Error> {
        let mut rx = self.wrapper.rx.borrow_mut();
        rx.receive().await
    }
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
    type Error = Infallible;

    type TX<'a> = Self
    where
        Self: 'a;

    type RX<'a> = Self
    where
        Self: 'a;

    fn split(&mut self) -> (Self, Self) {
        (self.clone(), self.clone())
    }
}

impl<D: Delay> CanBusTX for DummyCanBus<D> {
    type Error = Infallible;

    fn configure_self_node(&mut self, _node_type: u8, _node_id: u16) {
        // noop
    }

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
    type Error = Infallible;
    type Message = DummyCanBusMessage;

    async fn receive(&mut self) -> Result<Self::Message, Self::Error> {
        loop {
            self.delay.delay_ms(1000.0).await;
        }
    }
}

pub struct DummyCanBusMessage;

impl CanBusRawMessage for DummyCanBusMessage {
    fn timestamp(&self) -> f64 {
        todo!()
    }

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
