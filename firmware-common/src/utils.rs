use core::future::Future;

use futures::future::{select, Either};
use futures::pin_mut;

use crate::driver::timer::Timer;

#[macro_export]
macro_rules! try_or_warn {
    ($e: expr) => {{
        if let Err(e) = $e {
            defmt::warn!("`{}` failed: {:?}", stringify!($e), e);
        }
    }};
}

pub async fn run_with_timeout<F: Future>(
    timer: impl Timer,
    ms: f64,
    future: F,
) -> Result<F::Output, f64> {
    let timeout_fut = timer.sleep(ms);
    pin_mut!(timeout_fut);
    pin_mut!(future);
    match select(timeout_fut, future).await {
        Either::Left(_) => Err(ms),
        Either::Right((result, _)) => Ok(result),
    }
}
