pub use crate::common::vlp::application_layer::RadioApplicationClient;
pub use crate::common::vlp::application_layer::{
    ApplicationLayerRxPackage, ApplicationLayerTxPackage,
};
use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};
pub use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibratorState,
};
pub use ferraris_calibration::CalibrationInfo;

#[derive(Debug, Clone)]
pub enum DebuggerTargetEvent {
    Calibrating(InteractiveCalibratorState),
    ApplicationLayerPackage(ApplicationLayerTxPackage),
}

pub trait Debugger: Clone {
    type ApplicationLayer: RadioApplicationClient;
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

impl RadioApplicationClient for DummyApplicationLayer {
    type Error = ();

    async fn run<'a, 'b, R: RawMutex, const N: usize, const M: usize>(
        &mut self,
        _radio_tx: Receiver<'a, R, ApplicationLayerTxPackage, N>,
        _radio_rx: Sender<'b, R, ApplicationLayerRxPackage, M>,
    ) -> ! {
        unimplemented!()
    }
}
