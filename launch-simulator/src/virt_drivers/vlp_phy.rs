use std::{
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use bevy::prelude::Component;
use firmware_common::vlp::{
    application_layer::{
        ApplicationLayerRxPackage, ApplicationLayerTxPackage, RadioApplicationPackage,
    },
    phy::VLPPhy,
    Priority, VLPSocket,
};
use futures::{
    future::{select, Either},
    pin_mut,
};
use heapless::Vec;
use lora_phy::mod_params::RadioError;
use std::sync::mpsc::channel;
use tokio::sync::watch::{self, Receiver as AsyncReceiver, Sender as AsyncSender};
use tokio::time::sleep;
pub fn create_mock_phy_participants() -> (MockPhyParticipant, MockPhyParticipant) {
    let (tx_a, rx_a) = watch::channel(Vec::<u8, 222>::new());
    let (tx_b, rx_b) = watch::channel(Vec::<u8, 222>::new());

    (
        MockPhyParticipant { tx: tx_a, rx: rx_b },
        MockPhyParticipant { tx: tx_b, rx: rx_a },
    )
}

pub struct MockPhyParticipant {
    tx: AsyncSender<Vec<u8, 222>>,
    rx: AsyncReceiver<Vec<u8, 222>>,
}

impl VLPPhy for MockPhyParticipant {
    async fn tx(&mut self, payload: &[u8]) {
        sleep(Duration::from_millis(500)).await;
        self.tx.send(Vec::from_slice(payload).unwrap());
    }

    async fn rx(&mut self) -> Result<Vec<u8, 222>, RadioError> {
        self.rx.changed().await;
        Ok(self.rx.borrow().clone())
    }

    async fn rx_with_timeout(&mut self, _timeout_ms: u32) -> Result<Vec<u8, 222>, RadioError> {
        let rxfut = self.rx();
        pin_mut!(rxfut);
        let sleep_fut = sleep(Duration::from_millis(_timeout_ms as u64));
        pin_mut!(sleep_fut);
        match select(sleep_fut, rxfut).await {
            Either::Left(_) => Err(RadioError::ReceiveTimeout),
            Either::Right((x, _)) => x,
        }
    }

    fn increment_frequency(&mut self) {}

    fn reset_frequency(&mut self) {}
}

#[derive(Component)]
pub struct VLPClient {
    rx: Arc<Mutex<Receiver<ApplicationLayerTxPackage>>>,
    tx: Arc<Mutex<Sender<ApplicationLayerRxPackage>>>,
}

impl VLPClient {
    pub fn send(&self, package: ApplicationLayerRxPackage) {
        self.tx.as_ref().lock().unwrap().send(package).unwrap();
    }

    pub fn try_receive(&self) -> Option<ApplicationLayerTxPackage> {
        self.rx.as_ref().lock().unwrap().try_recv().ok()
    }
}

pub fn start_vlp_thread(phy: MockPhyParticipant) -> VLPClient {
    let (tx_a, rx_a) = channel();
    let (tx_b, rx_b) = channel();

    thread::spawn(move || {
        vlp_task(phy, rx_a, tx_b);
    });

    VLPClient {
        rx: Arc::new(Mutex::new(rx_b)),
        tx: Arc::new(Mutex::new(tx_a)),
    }
}

#[tokio::main]
async fn vlp_task(
    phy: MockPhyParticipant,
    radio_tx: Receiver<ApplicationLayerRxPackage>,
    radio_rx: Sender<ApplicationLayerTxPackage>,
) {
    let mut vlp = VLPSocket::await_establish(phy).await.unwrap();

    loop {
        match vlp.prio {
            Priority::Driver => {
                if let Ok(tx_package) = radio_tx.try_recv() {
                    vlp.transmit(tx_package.encode()).await;
                } else {
                    vlp.handoff().await;
                }
            }
            Priority::Listener => {
                if let Ok(Some(data)) = vlp.receive().await {
                    if let Some(decoded) = ApplicationLayerTxPackage::decode(data) {
                        radio_rx.send(decoded);
                    }
                }
            }
        }
    }
}
