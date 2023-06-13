use std::sync::{Arc, Mutex};

use bevy::prelude::Component;
use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};
use firmware_common::driver::debugger::{
    ApplicationLayerRxPackage, ApplicationLayerTxPackage, Debugger as DebuggerDriver,
    DebuggerTargetEvent, RadioApplicationClient,
};
use tokio::join;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
#[derive(Component)]
pub struct DebuggerHost {
    tx: UnboundedSender<ApplicationLayerRxPackage>,
    rx: UnboundedReceiver<DebuggerTargetEvent>,
}

impl DebuggerHost {
    pub fn try_recv(&mut self) -> Option<DebuggerTargetEvent> {
        self.rx.try_recv().ok()
    }

    pub fn send(&mut self, package: ApplicationLayerRxPackage) {
        self.tx.send(package).unwrap();
    }
}

#[derive(Clone)]
pub struct Debugger {
    tx: UnboundedSender<DebuggerTargetEvent>,
    rx: Arc<Mutex<Option<UnboundedReceiver<ApplicationLayerRxPackage>>>>,
}

impl DebuggerDriver for Debugger {
    type ApplicationLayer = DebuggerApplicationLayer;

    fn dispatch(&self, event: DebuggerTargetEvent) {
        self.tx.send(event).unwrap();
    }

    fn get_vlp_application_layer(&self) -> Option<Self::ApplicationLayer> {
        self.rx
            .lock()
            .unwrap()
            .take()
            .map(|rx| DebuggerApplicationLayer {
                tx: self.tx.clone(),
                rx,
            })
    }
}

pub struct DebuggerApplicationLayer {
    tx: UnboundedSender<DebuggerTargetEvent>,
    rx: UnboundedReceiver<ApplicationLayerRxPackage>,
}

impl RadioApplicationClient for DebuggerApplicationLayer {
    type Error = ();

    async fn run<'a, 'b, R: RawMutex, const N: usize, const M: usize>(
        &mut self,
        radio_tx: Receiver<'a, R, ApplicationLayerTxPackage, N>,
        radio_rx: Sender<'b, R, ApplicationLayerRxPackage, M>,
    ) -> ! {
        let send_fut = async {
            loop {
                let package = radio_tx.recv().await;
                self.tx
                    .send(DebuggerTargetEvent::ApplicationLayerPackage(package))
                    .unwrap();
            }
        };
        let recev_fut = async {
            loop {
                let package = self.rx.recv().await.unwrap();
                radio_rx.send(package).await;
            }
        };

        join!(send_fut, recev_fut);
        unreachable!()
    }
}

pub fn create_debugger() -> (Debugger, DebuggerHost) {
    let (tx, rx) = unbounded_channel::<DebuggerTargetEvent>();
    let (package_tx, package_rx) = unbounded_channel::<ApplicationLayerRxPackage>();
    (
        Debugger {
            tx,
            rx: Arc::new(Mutex::new(Some(package_rx))),
        },
        DebuggerHost { tx: package_tx, rx },
    )
}
