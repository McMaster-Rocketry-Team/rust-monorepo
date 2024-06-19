use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use embedded_hal_async::delay::DelayNs;

use crate::Buzzer;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BuzzerTone(
    u32, // frequency
    u32, // duration
    u32, // silent duration
);

pub struct BuzzerQueueRunner {
    channel: PubSubChannel<NoopRawMutex, BuzzerTone, 10, 1, 1>,
}

impl BuzzerQueueRunner {
    pub fn new() -> Self {
        let channel = PubSubChannel::<NoopRawMutex, BuzzerTone, 10, 1, 1>::new();

        Self { channel }
    }

    pub fn get_queue(&self) -> BuzzerQueue {
        BuzzerQueue {
            publisher: self.channel.publisher().unwrap(),
        }
    }

    pub async fn run(&self, mut buzzer: impl Buzzer, mut delay: impl DelayNs) -> ! {
        let mut sub = self.channel.subscriber().unwrap();
        loop {
            let tone = sub.next_message_pure().await;
            buzzer.play(tone.0, tone.1).await;
            delay.delay_ms(tone.2).await;
        }
    }
}

pub struct BuzzerQueue<'a> {
    publisher: Publisher<'a, NoopRawMutex, BuzzerTone, 10, 1, 1>,
}

impl BuzzerQueue<'_> {
    pub fn publish(&self, frequency: u32, duration: u32, silent_duration: u32) {
        self.publisher
            .publish_immediate(BuzzerTone(frequency, duration, silent_duration));
    }
}
