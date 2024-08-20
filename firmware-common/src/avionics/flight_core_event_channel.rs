use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

use super::flight_core_event::{FlightCoreEvent, FlightCoreEventPublisher};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightCoreRedundancy {
    Primary,
    Backup,
    BackupBackup,
}

pub struct FlightCoreEventChannel {
    channel: PubSubChannel<NoopRawMutex, (FlightCoreRedundancy, FlightCoreEvent), 10, 5, 3>,
}

impl FlightCoreEventChannel {
    pub fn new() -> Self {
        Self {
            channel: PubSubChannel::new(),
        }
    }

    pub fn subscriber(&self) -> Subscriber<NoopRawMutex, (FlightCoreRedundancy, FlightCoreEvent), 10, 5, 3> {
        self.channel.subscriber().unwrap()
    }

    pub fn publisher(&self, redundancy: FlightCoreRedundancy) -> FlightCoreEventChannelPublisher {
        FlightCoreEventChannelPublisher {
            publisher: self.channel.publisher().unwrap(),
            redundancy,
        }
    }
}

pub struct FlightCoreEventChannelPublisher<'a> {
    publisher: Publisher<'a, NoopRawMutex, (FlightCoreRedundancy, FlightCoreEvent), 10, 5, 3>,
    redundancy: FlightCoreRedundancy,
}

impl<'a> FlightCoreEventPublisher for FlightCoreEventChannelPublisher<'a> {
    fn publish(&mut self, event: FlightCoreEvent) {
        self.publisher.publish_immediate((self.redundancy, event));
    }
}
