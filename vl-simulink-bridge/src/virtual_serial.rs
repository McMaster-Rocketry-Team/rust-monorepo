use std::convert::Infallible;

use firmware_common::driver::serial::SplitableSerial;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};

pub struct VirtualSerial {
    tx: UnboundedSender<Vec<u8>>,
    rx: Mutex<UnboundedReceiver<Vec<u8>>>,
}

impl VirtualSerial {
    pub fn new() -> (Self, Self) {
        let (a_tx, b_rx) = unbounded_channel::<Vec<u8>>();
        let (b_tx, a_rx) = unbounded_channel::<Vec<u8>>();
        (
            VirtualSerial {
                tx: a_tx,
                rx: Mutex::new(a_rx),
            },
            VirtualSerial {
                tx: b_tx,
                rx: Mutex::new(b_rx),
            },
        )
    }
}

impl SplitableSerial for VirtualSerial {
    type Error = Infallible;

    fn split(
        &mut self,
    ) -> (
        impl embedded_io_async::Write<Error = Self::Error>,
        impl embedded_io_async::Read<Error = Self::Error>,
    ) {
        (
            (self as &VirtualSerial),
            VirtualSerialRx {
                serial: self,
                buffer: Vec::new(),
            },
        )
    }
}

impl embedded_io_async::ErrorType for &VirtualSerial {
    type Error = Infallible;
}

impl embedded_io_async::Write for &VirtualSerial {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.tx.send(Vec::from(buf)).unwrap();
        Ok(buf.len())
    }
}

struct VirtualSerialRx<'a> {
    serial: &'a VirtualSerial,
    buffer: Vec<u8>,
}

impl<'a> embedded_io_async::ErrorType for VirtualSerialRx<'a> {
    type Error = Infallible;
}

impl embedded_io_async::Read for VirtualSerialRx<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.buffer.len() > 0 {
            let len = std::cmp::min(self.buffer.len(), buf.len());
            buf[..len].copy_from_slice(&self.buffer[..len]);
            self.buffer = self.buffer.split_off(len);
            Ok(len)
        } else {
            let mut rx = self.serial.rx.lock().await;
            let mut data = rx.recv().await.unwrap();
            let len = std::cmp::min(data.len(), buf.len());
            buf[..len].copy_from_slice(&data[..len]);
            self.buffer = data.split_off(len);
            Ok(len)
        }
    }
}
