use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Sender};

use crate::common::vlp::telemetry_packet::FlightCoreStateTelemetry;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize, defmt::Format)]
pub enum FlightCoreEvent {
    CriticalError,
    Ignition,
    Apogee,
    DeployMain,
    DeployDrogue,
    Landed,
    DidNotReachMinApogee,
    ChangeState(FlightCoreStateTelemetry),
    ChangeAltitude(f32),
    ChangeSpeed(f32),
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
