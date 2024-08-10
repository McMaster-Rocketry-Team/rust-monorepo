use libm::ceil;

use crate::driver::{clock::Clock, delay::Delay};
pub struct Ticker<C: Clock, D: Delay> {
    start_timestamp: f64,
    interval_ms: f64,
    i: f64,
    clock: C,
    delay: D,
}

impl<C: Clock, D: Delay> Ticker<C, D> {
    pub fn every(clock: C, delay: D, interval_ms: f64) -> Self {
        Self {
            start_timestamp: clock.now_ms(),
            interval_ms,
            i: 0.0,
            clock,
            delay,
        }
    }

    pub fn every_starts_at(clock: C, delay: D, interval_ms: f64, start_timestamp: f64) -> Self {
        Self {
            start_timestamp,
            interval_ms,
            i: 0.0,
            clock,
            delay,
        }
    }

    pub async fn next(&mut self) {
        let now = self.clock.now_ms();
        self.i += 1.0;
        let wait_until_timestamp = self.start_timestamp + self.i * self.interval_ms;
        if now >= wait_until_timestamp {
            return;
        } else {
            self.delay.delay_ms(wait_until_timestamp - now).await;
        }
    }

    pub async fn next_skip_missed(&mut self) {
        let now = self.clock.now_ms();
        let wait_until_timestamp = ceil(now / self.interval_ms) * self.interval_ms;
        self.delay.delay_ms(wait_until_timestamp - now).await;
    }
}
