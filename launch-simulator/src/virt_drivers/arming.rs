use bevy::prelude::Component;
use firmware_common::driver::arming::HardwareArming;
use tokio::sync::watch::{self, Receiver, Sender};

pub struct VirtualHardwareArming {
    rx: Receiver<bool>,
}

impl HardwareArming for VirtualHardwareArming {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error> {
        self.rx.changed().await.unwrap();
        self.read_arming().await
    }

    async fn read_arming(&mut self) -> Result<bool, Self::Error> {
        Ok(self.rx.borrow().clone())
    }
}

#[derive(Component)]
pub struct VirtualHardwareArmingController{
    tx: Sender<bool>,
}

impl VirtualHardwareArmingController {
    pub fn set_arming(&mut self, arming: bool) {
        self.tx.send(arming).unwrap();
    }
}

pub fn create_hardware_arming() -> (VirtualHardwareArming, VirtualHardwareArmingController) {
    let (tx, rx) = watch::channel(false);
    (VirtualHardwareArming { rx }, VirtualHardwareArmingController { tx })
}
