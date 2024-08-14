use core::cell::RefCell;
use core::fmt;
use core::marker::PhantomData;
use embedded_hal_async::delay::DelayNs;

pub trait SplitableSerial: fmt::Debug {
    type Error: defmt::Format + embedded_io_async::Error + fmt::Debug;
    type TX<'a>: embedded_io_async::Write<Error = Self::Error>
    where
        Self: 'a;
    type RX<'a>: embedded_io_async::Read<Error = Self::Error>
    where
        Self: 'a;

    fn split(&mut self) -> (Self::TX<'_>, Self::RX<'_>);
}

// TODO move E after T and R
pub struct SplitableSerialWrapper<
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    _phantom_data: PhantomData<E>,
    tx: RefCell<T>,
    rx: RefCell<R>,
}

impl<
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > fmt::Debug for SplitableSerialWrapper<E, T, R>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SplitableSerialWrapper")
    }
}

impl<
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > SplitableSerialWrapper<E, T, R>
{
    pub fn new(tx: T, rx: R) -> Self {
        Self {
            _phantom_data: PhantomData,
            tx: RefCell::new(tx),
            rx: RefCell::new(rx),
        }
    }
}

impl<
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > SplitableSerial for SplitableSerialWrapper<E, T, R>
{
    type Error = E;
    type TX<'a> = TXGuard<'a, E, T, R> where Self: 'a;
    type RX<'a> = RXGuard<'a, E, T, R> where Self: 'a;

    fn split(&mut self) -> (TXGuard<'_, E, T, R>, RXGuard<'_, E, T, R>) {
        (TXGuard { wrapper: self }, RXGuard { wrapper: self })
    }
}

pub struct TXGuard<
    'a,
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    wrapper: &'a SplitableSerialWrapper<E, T, R>,
}

impl<
        'a,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::ErrorType for TXGuard<'a, E, T, R>
{
    type Error = E;
}

impl<
        'a,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::Write for TXGuard<'a, E, T, R>
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut tx = self.wrapper.tx.borrow_mut();
        tx.write(buf).await
    }
}

pub struct RXGuard<
    'a,
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    wrapper: &'a SplitableSerialWrapper<E, T, R>,
}
impl<
        'a,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::ErrorType for RXGuard<'a, E, T, R>
{
    type Error = E;
}
impl<
        'a,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::Read for RXGuard<'a, E, T, R>
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut rx = self.wrapper.rx.borrow_mut();
        rx.read(buf).await
    }
}

pub fn get_dummy_serial(delay: impl DelayNs) -> impl SplitableSerial {
    SplitableSerialWrapper::new(DummyTX, DummyRX { delay })
}

#[derive(Debug, defmt::Format)]
struct DummySerialError;

impl embedded_io_async::Error for DummySerialError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Unsupported
    }
}

struct DummyTX;

impl embedded_io_async::ErrorType for DummyTX {
    type Error = DummySerialError;
}

impl embedded_io_async::Write for DummyTX {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(buf.len())
    }
}

struct DummyRX<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> embedded_io_async::ErrorType for DummyRX<D> {
    type Error = DummySerialError;
}

impl<D: DelayNs> embedded_io_async::Read for DummyRX<D> {
    async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            self.delay.delay_ms(1000).await;
        }
    }
}
