pub use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibratorState,
};

#[derive(Debug, Clone)]
pub enum DebuggerEvent {
    CalibrationStart,
    Calibrating(InteractiveCalibratorState),
}

pub trait Debugger: Copy {
    fn dispatch(&self, event: DebuggerEvent);
}

#[derive(Clone, Copy)]
pub struct DummyDebugger {}

impl Debugger for DummyDebugger {
    fn dispatch(&self, _event: DebuggerEvent) {}
}
