use core::cmp::min;

use super::serial::Serial;

pub trait USB {
    async fn write_64b(&mut self, data: &[u8]) -> Result<(), ()>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()>;
}

impl<T: USB> Serial for T {
    async fn write(&mut self, data: &[u8]) -> Result<(), ()> {
        for i in (0..data.len()).step_by(64) {
            let end = min(i + 64, data.len());
            self.write_64b(&data[i..end]).await?;
        }
        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()> {
        self.read(buffer).await
    }
}
