use core::cmp::min;

use super::serial::Serial;

pub trait USB {
    type Error: defmt::Format;
    async fn write_64b(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
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
