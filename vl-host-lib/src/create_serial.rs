use firmware_common::driver::serial::SplitableSerialWrapper;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};

use anyhow::Result;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[derive(defmt::Format, Debug)]
pub struct SerialErrorWrapper(#[defmt(Debug2Format)] std::io::Error);

impl embedded_io_async::Error for SerialErrorWrapper {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Other
    }
}

pub struct SerialRXWrapper(ReadHalf<SerialStream>);

impl embedded_io_async::ErrorType for SerialRXWrapper {
    type Error = SerialErrorWrapper;
}

impl embedded_io_async::Read for SerialRXWrapper {
    async fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(SerialErrorWrapper)
    }
}

pub struct SerialTXWrapper(WriteHalf<SerialStream>);

impl embedded_io_async::ErrorType for SerialTXWrapper {
    type Error = SerialErrorWrapper;
}

impl embedded_io_async::Write for SerialTXWrapper {
    async fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, Self::Error> {
        self.0.write(buf).await.map_err(SerialErrorWrapper)
    }
}

pub fn create_serial(
    serial_port_name: String,
) -> Result<SplitableSerialWrapper<SerialErrorWrapper, SerialTXWrapper, SerialRXWrapper>> {
    let serial: tokio_serial::SerialStream =
        tokio_serial::new(serial_port_name, 115200).open_native_async()?;
    let (rx, tx) = split(serial);
    Ok(SplitableSerialWrapper::new(
        SerialTXWrapper(tx),
        SerialRXWrapper(rx),
    ))
}
