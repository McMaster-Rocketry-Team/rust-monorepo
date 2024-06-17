use embedded_hal_async::delay::DelayNs;

use crate::Clock;

pub struct Ticker<C: Clock, D: DelayNs> {
    expires_at_ms: f64,
    pub interval_ms: f64,
    clock: C,
    delay: D,
}

impl<C: Clock, D: DelayNs> Ticker<C, D> {
    pub fn every(clock: C, delay: D, interval_ms: f64) -> Self {
        let expires_at_ms = clock.now_ms() + interval_ms;
        Self {
            expires_at_ms,
            interval_ms,
            clock,
            delay,
        }
    }

    pub async fn next(&mut self) -> f64 {
        let now = self.clock.now_ms();
        let elapsed = now - self.expires_at_ms + self.interval_ms;
        if now > self.expires_at_ms {
            self.expires_at_ms = now + self.interval_ms;
        } else {
            self.delay
                .delay_us(((self.expires_at_ms - now) * 1000.0) as u32)
                .await;
            self.expires_at_ms += self.interval_ms;
        }
        elapsed
    }
}
