use embedded_hal_async::delay::DelayUs;

pub trait Timer: Copy {
    async fn sleep(&self, ms: u64);
    fn now_mills(&self) -> u64;
    fn now_micros(&self) -> u64;
}

pub(crate) struct DelayUsWrapper<T: Timer>(pub(crate) T);

impl<T: Timer> DelayUs for DelayUsWrapper<T> {
    async fn delay_us(&mut self, us: u32) {
        self.0.sleep((us as u64 / 1000).min(1)).await;
    }

    async fn delay_ms(&mut self, ms: u32) {
        self.0.sleep(ms as u64).await;
    }
}
