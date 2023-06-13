use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Sender};

pub enum FlightCoreEvent {
    CriticalError,
    Ignition,
    Apogee,
    DeployMain,
    DeployDrogue,
    Landed,
    DidNotReachMinApogee,
}

pub trait FlightCoreEventDispatcher {
    fn dispatch(&mut self, event: FlightCoreEvent);
}

impl<'ch, M: RawMutex, const N: usize> FlightCoreEventDispatcher
    for Sender<'ch, M, FlightCoreEvent, N>
{
    fn dispatch(&mut self, event: FlightCoreEvent) {
        if self.try_send(event).is_err() {
            log_warn!("FlightCoreEventDispatcher: event queue full");
        }
    }
}
