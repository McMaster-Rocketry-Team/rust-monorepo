use embedded_hal_async::delay::DelayUs;
use vlfs::Timer as VLFSTimer;

pub trait Timer: Copy {
    async fn sleep(&self, ms: f64);
    fn now_mills(&self) -> f64;
}

pub struct DelayUsWrapper<T: Timer>(pub T);

impl<T: Timer> DelayUs for DelayUsWrapper<T> {
    async fn delay_us(&mut self, us: u32) {
        self.0.sleep((us as f64 / 1000.0).min(1.0)).await;
    }

    async fn delay_ms(&mut self, ms: u32) {
        self.0.sleep(ms as f64).await;
    }
}

#[derive(Copy, Clone)]
pub(crate) struct VLFSTimerWrapper<T: Timer>(pub(crate) T);

impl<T: Timer> VLFSTimer for VLFSTimerWrapper<T> {
    async fn sleep(&self, ms: f64) {
        self.0.sleep(ms).await;
    }

    fn now_mills(&self) -> f64 {
        self.0.now_mills()
    }
}
