use crate::avionics::flight_core_event::FlightCoreEvent;

pub use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibratorState,
};
pub use ferraris_calibration::CalibrationInfo;

#[derive(Debug, Clone)]
pub enum DebuggerTargetEvent {
    Calibrating(InteractiveCalibratorState),
    FlightCoreEvent(FlightCoreEvent),
}

pub trait Debugger: Clone {
    fn dispatch(&self, event: DebuggerTargetEvent);
}

#[derive(Clone)]
pub struct DummyDebugger {}

impl Debugger for DummyDebugger {
    fn dispatch(&self, _event: DebuggerTargetEvent) {}
}
