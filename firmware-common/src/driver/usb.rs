use core::cmp::min;

use super::{serial::Serial, timer::Timer};

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

pub struct DummyUSB<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyUSB<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> USB for DummyUSB<T> {
    type Error = ();

    async fn write_64b(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }

    async fn wait_connection(&mut self) {
        loop {
            self.timer.sleep(1000.0).await;
        }
    }
}
