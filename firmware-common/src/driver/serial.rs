use core::cell::RefCell;
use core::marker::PhantomData;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embedded_hal_async::delay::DelayNs;
use embedded_io_async::ReadExactError;

pub trait SplitableSerial {
    type Error: defmt::Format + embedded_io_async::Error;

    fn split(
        &mut self,
    ) -> (
        impl embedded_io_async::Write<Error = Self::Error>,
        impl embedded_io_async::Read<Error = Self::Error>,
    );
}

pub struct SplitableSerialWrapper<
    M: RawMutex,
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    _phantom_data: PhantomData<E>,
    tx: BlockingMutex<M, RefCell<Option<T>>>,
    rx: BlockingMutex<M, RefCell<Option<R>>>,
}

impl<
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > SplitableSerialWrapper<M, E, T, R>
{
    pub fn new(tx: T, rx: R) -> Self {
        Self {
            _phantom_data: PhantomData,
            tx: BlockingMutex::new(RefCell::new(Some(tx))),
            rx: BlockingMutex::new(RefCell::new(Some(rx))),
        }
    }
}

impl<
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > SplitableSerial for SplitableSerialWrapper<M, E, T, R>
{
    type Error = E;

    fn split(&mut self) -> (TXGuard<'_, M, E, T, R>, RXGuard<'_, M, E, T, R>) {
        (
            TXGuard {
                wrapper: self,
                tx: self.tx.lock(|tx| tx.borrow_mut().take()),
            },
            RXGuard {
                wrapper: self,
                rx: self.rx.lock(|rx| rx.borrow_mut().take()),
            },
        )
    }
}

pub struct TXGuard<
    'a,
    M: RawMutex,
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    wrapper: &'a SplitableSerialWrapper<M, E, T, R>,
    tx: Option<T>,
}

impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::ErrorType for TXGuard<'a, M, E, T, R>
{
    type Error = E;
}

impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::Write for TXGuard<'a, M, E, T, R>
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.tx.as_mut().unwrap().write(buf).await
    }
}

impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > Drop for TXGuard<'a, M, E, T, R>
{
    fn drop(&mut self) {
        self.wrapper.tx.lock(|tx| {
            *tx.borrow_mut() = self.tx.take();
        });
    }
}

pub struct RXGuard<
    'a,
    M: RawMutex,
    E: defmt::Format + embedded_io_async::Error,
    T: embedded_io_async::Write<Error = E>,
    R: embedded_io_async::Read<Error = E>,
> {
    wrapper: &'a SplitableSerialWrapper<M, E, T, R>,
    rx: Option<R>,
}
impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::ErrorType for RXGuard<'a, M, E, T, R>
{
    type Error = E;
}
impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > embedded_io_async::Read for RXGuard<'a, M, E, T, R>
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.rx.as_mut().unwrap().read(buf).await
    }
}
impl<
        'a,
        M: RawMutex,
        E: defmt::Format + embedded_io_async::Error,
        T: embedded_io_async::Write<Error = E>,
        R: embedded_io_async::Read<Error = E>,
    > Drop for RXGuard<'a, M, E, T, R>
{
    fn drop(&mut self) {
        self.wrapper.rx.lock(|rx| {
            *rx.borrow_mut() = self.rx.take();
        });
    }
}

pub fn get_dummy_serial(delay: impl DelayNs) -> impl SplitableSerial {
    SplitableSerialWrapper::<NoopRawMutex, _, _, _>::new(DummyTX, DummyRX { delay })
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
