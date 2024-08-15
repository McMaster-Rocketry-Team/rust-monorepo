use std::convert::Infallible;

use tokio::io::AsyncReadExt;
use tokio::io::{AsyncRead, BufReader};

pub struct BufReaderWrapper<R: AsyncRead>(pub BufReader<R>);

impl<R: AsyncRead + Unpin> embedded_io_async::ErrorType for BufReaderWrapper<R> {
    type Error = Infallible;
}

impl<R: AsyncRead + Unpin> embedded_io_async::Read for BufReaderWrapper<R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(self.0.read(buf).await.unwrap())
    }
}
