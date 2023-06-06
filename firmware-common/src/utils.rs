use core::future::Future;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::signal::Signal;
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

pub struct Debouncer<M: RawMutex, V: Send, T: Timer> {
    signal: Signal<M, (V, f64)>,
    duration_ms: f64,
    timer: T,
}

impl<M: RawMutex, V: Send, T: Timer> Debouncer<M, V, T> {
    pub fn new(timer: T, duration_ms: f64) -> Self {
        Self {
            signal: Signal::new(),
            duration_ms,
            timer,
        }
    }

    pub fn put(&self, value: V) {
        self.signal.signal((value, self.timer.now_mills()));
    }

    pub async fn wait_next(&self) -> V {
        let mut value = self.signal.wait().await;
        if value.1 + self.duration_ms < self.timer.now_mills() {
            return value.0;
        }
        loop {
            match run_with_timeout(
                self.timer,
                self.duration_ms - (self.timer.now_mills() - value.1),
                self.signal.wait(),
            )
            .await
            {
                Ok(v) => {
                    value = v;
                }
                Err(_) => {
                    return value.0;
                }
            }
        }
    }
}
