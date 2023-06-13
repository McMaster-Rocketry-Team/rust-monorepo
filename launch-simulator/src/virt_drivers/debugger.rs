use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::Component;
use firmware_common::driver::debugger::{
    ApplicationLayerPackage, Debugger as DebuggerDriver, DebuggerTargetEvent, RadioApplicationLayer,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::timeout;

#[derive(Component)]
pub struct DebuggerHost {
    tx: UnboundedSender<ApplicationLayerPackage>,
    rx: UnboundedReceiver<DebuggerTargetEvent>,
}

impl DebuggerHost {
    pub fn try_recv(&mut self) -> Option<DebuggerTargetEvent> {
        self.rx.try_recv().ok()
    }

    pub fn send(&mut self, package: ApplicationLayerPackage) {
        self.tx.send(package).unwrap();
    }
}

#[derive(Clone)]
pub struct Debugger {
    tx: UnboundedSender<DebuggerTargetEvent>,
    rx: Arc<Mutex<Option<UnboundedReceiver<ApplicationLayerPackage>>>>,
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
    rx: UnboundedReceiver<ApplicationLayerPackage>,
}

impl RadioApplicationLayer for DebuggerApplicationLayer {
    type Error = ();

    async fn send(&mut self, package: ApplicationLayerPackage) -> Result<(), Self::Error> {
        self.tx
            .send(DebuggerTargetEvent::ApplicationLayerPackage(package))
            .unwrap();
        Ok(())
    }

    async fn receive(&mut self, timeout_ms: f64) -> Result<ApplicationLayerPackage, Self::Error> {
        if let Ok(Some(package)) =
            timeout(Duration::from_millis(timeout_ms as u64), self.rx.recv()).await
        {
            Ok(package)
        } else {
            Err(())
        }
    }
}

pub fn create_debugger() -> (Debugger, DebuggerHost) {
    let (tx, rx) = unbounded_channel::<DebuggerTargetEvent>();
    let (package_tx, package_rx) = unbounded_channel::<ApplicationLayerPackage>();
    (
        Debugger {
            tx,
            rx: Arc::new(Mutex::new(Some(package_rx))),
        },
        DebuggerHost { tx: package_tx, rx },
    )
}
