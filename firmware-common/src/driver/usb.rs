use core::cmp::min;

use super::serial::Serial;
use embedded_hal_async::delay::DelayNs;

pub trait USB {
    type Error: defmt::Format;
    async fn write_64b(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
    async fn wait_connection(&mut self);
}

impl<T: USB> Serial for T {
    type Error = T::Error;

    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        for i in (0..data.len()).step_by(64) {
            let end = min(i + 64, data.len());
            self.write_64b(&data[i..end]).await?;
        }
        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buffer).await
    }
}

pub struct DummyUSB<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyUSB<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> USB for DummyUSB<D> {
    type Error = ();

    async fn write_64b(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            self.delay.delay_ms(1000).await;
        }
    }

    async fn wait_connection(&mut self) {
        loop {
            self.delay.delay_ms(1000).await;
        }
    }
}
