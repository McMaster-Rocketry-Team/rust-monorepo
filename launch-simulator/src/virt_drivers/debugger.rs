use firmware_common::driver::debugger::{DebuggerEvent,Debugger as DebuggerDriver};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Clone)]
pub struct Debugger {
    tx: UnboundedSender<DebuggerEvent>,
}

impl DebuggerDriver for Debugger{
    fn dispatch(&self, event: DebuggerEvent) {
        self.tx.send(event).unwrap();
    }
}

pub fn create_debugger() -> (Debugger, UnboundedReceiver<DebuggerEvent>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (Debugger { tx }, rx)
}
