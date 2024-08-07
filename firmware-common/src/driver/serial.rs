use core::cell::RefCell;
use core::marker::PhantomData;
use embedded_hal_async::delay::DelayNs;
use embedded_io_async::ReadExactError;

pub trait SplitableSerial {
    type Error: defmt::Format + embedded_io_async::Error + core::fmt::Debug;
    // TODO use type: XXX instead of impl in return?

    fn split(
        &mut self,
    ) -> (
        impl embedded_io_async::Write<Error = Self::Error>,
        impl embedded_io_async::Read<Error = Self::Error>,
    );
}

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

    #[allow(refining_impl_trait_reachable)]
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

pub enum RpcClientError<S: crate::driver::serial::SplitableSerial> {
    Timeout,
    UnexpectedEof,
    Serial(S::Error),
}

impl<S: crate::driver::serial::SplitableSerial> From<ReadExactError<S::Error>>
    for RpcClientError<S>
{
    fn from(value: ReadExactError<S::Error>) -> Self {
        match value {
            ReadExactError::Other(e) => RpcClientError::Serial(e),
            ReadExactError::UnexpectedEof => RpcClientError::UnexpectedEof,
        }
    }
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
