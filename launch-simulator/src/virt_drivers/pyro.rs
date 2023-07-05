use bevy::prelude::Component;
use firmware_common::driver::pyro::PyroCtrl;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct VirtualPyro {
    tx: UnboundedSender<()>,
}

impl PyroCtrl for VirtualPyro {
    type Error = ();

    async fn set_enable(&mut self, enable: bool) -> Result<(), Self::Error> {
        if enable {
            self.tx.send(()).unwrap();
        }
        Ok(())
    }
}

#[derive(Component)]
pub struct PyroReceiver {
    rx: UnboundedReceiver<()>,
    pub pyro_channel: u8,
}

impl PyroReceiver {
    pub fn try_recv(&mut self) -> Option<()> {
        self.rx.try_recv().ok()
    }
}

pub fn create_pyro(pyro_channel: u8) -> (VirtualPyro, PyroReceiver) {
    let (tx, rx) = unbounded_channel();
    (VirtualPyro { tx }, PyroReceiver { rx, pyro_channel })
}
