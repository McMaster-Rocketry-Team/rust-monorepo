use embedded_hal_async::delay::DelayNs;
use core::future::Future;

pub trait Delay: DelayNs + Clone {
    fn delay_ms(&self, ms: f64) -> impl Future<Output = ()> + Send;
}
