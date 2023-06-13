pub use crate::common::vlp::application_layer::{ApplicationLayerPackage, RadioApplicationLayer};
pub use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibratorState,
};
pub use ferraris_calibration::CalibrationInfo;

#[derive(Debug, Clone)]
pub enum DebuggerTargetEvent {
    Calibrating(InteractiveCalibratorState),
    ApplicationLayerPackage(ApplicationLayerPackage),
}

pub trait Debugger: Clone {
    type ApplicationLayer: RadioApplicationLayer;
    fn dispatch(&self, event: DebuggerTargetEvent);
    fn get_vlp_application_layer(&self) -> Option<Self::ApplicationLayer>;
}

#[derive(Clone)]
pub struct DummyDebugger {}

impl Debugger for DummyDebugger {
    type ApplicationLayer = DummyApplicationLayer;
    fn dispatch(&self, _event: DebuggerTargetEvent) {}
    fn get_vlp_application_layer(&self) -> Option<Self::ApplicationLayer> {
        None
    }
}

pub struct DummyApplicationLayer {}

impl RadioApplicationLayer for DummyApplicationLayer {
    type Error = ();

    async fn send(&mut self, _package: ApplicationLayerPackage) -> Result<(), Self::Error> {
        unimplemented!()
    }

    async fn receive(&mut self, _timeout_ms: f64) -> Result<ApplicationLayerPackage, Self::Error> {
        unimplemented!()
    }
}
