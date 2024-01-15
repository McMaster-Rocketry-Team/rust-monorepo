use bevy::prelude::Component;
use firmware_common::driver::debugger::{Debugger as DebuggerDriver, DebuggerTargetEvent};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
#[derive(Component)]
pub struct DebuggerHost {
    rx: UnboundedReceiver<DebuggerTargetEvent>,
}

impl DebuggerHost {
    pub fn try_recv(&mut self) -> Option<DebuggerTargetEvent> {
        self.rx.try_recv().ok()
    }
}

#[derive(Clone)]
pub struct Debugger {
    tx: UnboundedSender<DebuggerTargetEvent>,
}

impl DebuggerDriver for Debugger {
    fn dispatch(&self, event: DebuggerTargetEvent) {
        self.tx.send(event).unwrap();
    }
}

pub fn create_debugger() -> (Debugger, DebuggerHost) {
    let (tx, rx) = unbounded_channel::<DebuggerTargetEvent>();
    (Debugger { tx }, DebuggerHost { rx })
}
