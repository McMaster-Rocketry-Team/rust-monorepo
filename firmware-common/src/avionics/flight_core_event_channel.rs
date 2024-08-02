use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

use super::flight_core_event::{FlightCoreEvent, FlightCoreEventPublisher};

pub struct FlightCoreEventChannel {
    channel: PubSubChannel<NoopRawMutex, (bool, FlightCoreEvent), 10, 5, 2>,
}

impl FlightCoreEventChannel {
    pub fn new() -> Self {
        Self {
            channel: PubSubChannel::new(),
        }
    }

    pub fn subscriber(&self) -> Subscriber<NoopRawMutex, (bool, FlightCoreEvent), 10, 5, 2> {
        self.channel.subscriber().unwrap()
    }

    pub fn publisher(&self, is_backup: bool) -> FlightCoreEventChannelPublisher {
        FlightCoreEventChannelPublisher {
            publisher: self.channel.publisher().unwrap(),
            is_backup,
        }
    }
}

pub struct FlightCoreEventChannelPublisher<'a> {
    publisher: Publisher<'a, NoopRawMutex, (bool, FlightCoreEvent), 10, 5, 2>,
    is_backup: bool,
}

impl<'a> FlightCoreEventPublisher for FlightCoreEventChannelPublisher<'a> {
    fn publish(&mut self, event: FlightCoreEvent) {
        self.publisher.publish_immediate((self.is_backup, event));
    }
}
