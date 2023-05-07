use core::ops::DerefMut;

use embassy_sync::{mutex::MutexGuard, blocking_mutex::raw::RawMutex};


// TODO define error type
pub trait Serial {
    async fn write(&mut self, data: &[u8]) -> Result<(), ()>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()>;

    async fn writeln(&mut self, data: &[u8]) -> Result<(), ()> {
        self.write(data).await?;
        self.write(b"\r\n").await?;
        Ok(())
    }

    async fn read_all(&mut self, buffer: &mut [u8])->Result<(),()>{
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
    async fn write(&mut self, data: &[u8]) -> Result<(), ()> {
        self.deref_mut().write(data).await
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()> {
        self.deref_mut().read(buffer).await
    }
}