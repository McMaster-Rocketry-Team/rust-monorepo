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
