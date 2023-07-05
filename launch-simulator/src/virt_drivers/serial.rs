use bevy::prelude::Component;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use firmware_common::driver::serial::Serial;

#[derive(Component)]
pub struct VirtualSerial {
    tx: UnboundedSender<Vec<u8>>,
    rx: UnboundedReceiver<Vec<u8>>,
}

pub fn create_virtual_serial() -> (VirtualSerial, VirtualSerial) {
    let (a_tx, b_rx) = unbounded_channel::<Vec<u8>>();
    let (b_tx, a_rx) = unbounded_channel::<Vec<u8>>();
    (
        VirtualSerial { tx: a_tx, rx: a_rx },
        VirtualSerial { tx: b_tx, rx: b_rx },
    )
}

impl VirtualSerial {
    pub fn blocking_write(&mut self, data: &[u8]) {
        self.tx.send(Vec::from(data)).unwrap();
    }

    pub fn try_read(&mut self) -> Option<Vec<u8>> {
        self.rx.try_recv().ok()
    }
}

impl Serial for VirtualSerial {
    type Error = ();

    async fn write(&mut self, data: &[u8]) -> Result<(), ()> {
        self.tx.send(Vec::from(data)).unwrap();
        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()> {
        self.rx
            .recv()
            .await
            .map(|data| {
                buffer[..data.len()].copy_from_slice(&data);
                data.len()
            })
            .ok_or(())
    }
}
