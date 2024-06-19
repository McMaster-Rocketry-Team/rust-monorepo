use embedded_hal_async::delay::DelayNs;

pub trait Delay: DelayNs + Copy {
    async fn delay_ms(&self, ms: u32);
}
