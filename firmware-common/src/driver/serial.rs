use core::ops::DerefMut;

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

use embedded_hal_async::delay::DelayNs;

pub trait Serial {
    type Error: defmt::Format;

    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    async fn writeln(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.write(data).await?;
        self.write(b"\r\n").await?;
        Ok(())
    }

    async fn read_all(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let mut read_length = 0;
        while read_length < buffer.len() {
            read_length += self.read(&mut buffer[read_length..]).await?;
        }

        Ok(())
    }
}

impl<'a, M, T> Serial for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Serial,
{
    type Error = T::Error;

    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.deref_mut().write(data).await
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.deref_mut().read(buffer).await
    }
}

pub struct DummySerial<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummySerial<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Serial for DummySerial<D> {
    type Error = ();

    async fn write(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            self.delay.delay_ms(1000).await;
        }
    }
}
