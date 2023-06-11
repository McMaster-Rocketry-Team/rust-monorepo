use firmware_common::driver::timer::Timer as TimerDriver;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
#[derive(Copy, Clone)]
struct TokioTimer {}

impl TimerDriver for TokioTimer {
    async fn sleep(&self, ms: f64) {
        sleep(Duration::from_micros((ms / 1000.0) as u64)).await;
    }

    fn now_mills(&self) -> f64 {
        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
        since_the_epoch.as_secs_f64() * 1000.0
    }
}
