use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Sender as ChannelSender, pubsub::publisher::Publisher as PubSubPublisher};

use int_enum::IntEnum;
use rkyv::{Archive, Deserialize, Serialize};

#[repr(u8)]
#[derive(defmt::Format, Debug, Clone, Copy, IntEnum, Archive, Deserialize, Serialize, PartialEq)]
pub enum FlightCoreState {
    DisArmed = 0,
    Armed = 1,
    PowerAscend = 2,
    Coast = 3,
    Descent = 4,
    DrogueChuteDeployed = 5,
    MainChuteDeployed = 6,
    Landed = 7,
}

#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize, defmt::Format, PartialEq)]
pub enum FlightCoreEvent {
    CriticalError,
    DidNotReachMinApogee,
    ChangeState(FlightCoreState),
    ChangeAltitude(f32),
    ChangeAirSpeed(f32),
}

pub trait FlightCoreEventPublisher {
    fn publish(&mut self, event: FlightCoreEvent);
}

impl<'ch, M: RawMutex, const N: usize> FlightCoreEventPublisher
    for ChannelSender<'ch, M, FlightCoreEvent, N>
{
    fn publish(&mut self, event: FlightCoreEvent) {
        if self.try_send(event).is_err() {
            log_warn!("FlightCoreEventDispatcher: event queue full");
        }
    }
}

impl<'a, M: RawMutex, const CAP: usize, const SUBS: usize, const PUBS: usize> FlightCoreEventPublisher
    for PubSubPublisher<'a, M, FlightCoreEvent, CAP, SUBS, PUBS>
{
    fn publish(&mut self, event: FlightCoreEvent) {
        self.publish_immediate(event)
    }
}
