use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use embedded_hal_async::delay::DelayNs;

use crate::Buzzer;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzzerTone(pub Option<u32>, pub u32);

pub struct BuzzerQueueRunner {
    channel: PubSubChannel<NoopRawMutex, BuzzerTone, 10, 1, 1>
}

impl BuzzerQueueRunner {
    pub fn new() -> Self {
        let channel = PubSubChannel::<NoopRawMutex, BuzzerTone, 10, 1, 1>::new();
        
        Self {channel}
    }

    pub fn get_queue(&self) -> BuzzerQueue {
        BuzzerQueue {
            publisher: self.channel.publisher().unwrap()
        }
    }

    pub async fn run(&self, mut buzzer: impl Buzzer, mut delay: impl DelayNs) -> !{
        let mut sub = self.channel.subscriber().unwrap();
        loop {
            let tone = sub.next_message_pure().await;
            if let Some(frequency) = tone.0 {
                buzzer.play(frequency, tone.1).await;
            } else {
                delay.delay_ms(tone.1).await;
            }
        }
    }
}


pub struct BuzzerQueue<'a> {
    publisher: Publisher<'a,NoopRawMutex, BuzzerTone, 10, 1, 1>
}

impl BuzzerQueue<'_> {
    pub fn publish(&self, tone: BuzzerTone) {
        self.publisher.publish_immediate(tone);
    }
}
