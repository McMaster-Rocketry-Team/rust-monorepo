use crate::driver::timer::Timer;

pub struct Ticker<T: Timer> {
    expires_at_ms: f64,
    duration_ms: f64,
    timer: T,
}

impl<T: Timer> Ticker<T> {
    pub fn every(timer: T, duration_ms: f64) -> Self {
        let expires_at_ms = timer.now_mills() + duration_ms;
        Self {
            expires_at_ms,
            duration_ms,
            timer,
        }
    }

    pub async fn next(&mut self) -> f64 {
        let now = self.timer.now_mills();
        let elapsed = now - self.expires_at_ms + self.duration_ms;
        if now > self.expires_at_ms {
            self.expires_at_ms = now + self.duration_ms;
        } else {
            self.timer.sleep(self.expires_at_ms - now).await;
            self.expires_at_ms += self.duration_ms;
        }
        elapsed
    }
}
