use embedded_hal_async::delay::DelayNs;

pub trait Delay: DelayNs + Clone {
    async fn delay_ms(&self, ms: f64);
}
