use std::convert::Infallible;

use tokio::io::AsyncReadExt;
use tokio::io::{AsyncRead, BufReader};
use tokio::pin;
pub struct VecReader {
    pub buffer: Vec<u8>,
    pub offset: usize,
}

impl VecReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, offset: 0 }
    }

    fn data_left(&self) -> usize {
        self.buffer.len() - self.offset
    }
}

impl embedded_io_async::ErrorType for VecReader {
    type Error = Infallible;
}

impl embedded_io_async::Read for VecReader {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.data_left() == 0 {
            Ok(0)
        } else if buf.len() > self.data_left() {
            let len = self.data_left();
            (&mut buf[..len]).copy_from_slice(&self.buffer[self.offset..self.buffer.len()]);
            self.offset = self.buffer.len();
            Ok(len)
        } else {
            buf.copy_from_slice(&self.buffer[self.offset..self.offset + buf.len()]);
            self.offset += buf.len();
            Ok(buf.len())
        }
    }
}

pub struct BufReaderWrapper<R: AsyncRead>(pub BufReader<R>);

impl<R: AsyncRead + Unpin> embedded_io_async::ErrorType for BufReaderWrapper<R> {
    type Error = Infallible;
}

impl<R: AsyncRead + Unpin> embedded_io_async::Read for BufReaderWrapper<R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(self.0.read(buf).await.unwrap())
    }
}
