use core::future::Future;

use embedded_hal_async::delay::DelayNs;
use futures::future::{select, Either};
use futures::pin_mut;

pub async fn run_with_timeout<F: Future>(
    mut delay: impl DelayNs,
    ms: f64,
    future: F,
) -> Result<F::Output, f64> {
    let timeout_fut = delay.delay_us((ms*1000.0) as u32);
    pin_mut!(timeout_fut);
    pin_mut!(future);
    match select(timeout_fut, future).await {
        Either::Left(_) => Err(ms),
        Either::Right((result, _)) => Ok(result),
    }
}
