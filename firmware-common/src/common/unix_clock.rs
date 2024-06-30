use core::{cell::RefCell, future::poll_fn, task::Poll};

use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};

use crate::{driver::gps::GPSPPS, Clock};

use super::{gps_parser::GPSParser, moving_average::NoSumSMA, multi_waker::MultiWakerRegistration};

#[derive(Default)]
struct UnixClockState {
    offset: Option<f64>,
    waker: MultiWakerRegistration<10>,
}

pub struct UnixClockTask<K: Clock> {
    state: BlockingMutex<NoopRawMutex, RefCell<UnixClockState>>,
    clock: K,
}

impl<K: Clock> UnixClockTask<K> {
    pub fn new(clock: K) -> Self {
        Self {
            state: BlockingMutex::new(RefCell::new(UnixClockState::default())),
            clock,
        }
    }

    pub fn get_clock(&self) -> UnixClock<K> {
        UnixClock::new(self)
    }

    pub async fn run(&self, mut pps: impl GPSPPS, gps_parser: &GPSParser, clock: impl Clock) -> ! {
        let mut offset_running_avg = NoSumSMA::<f64, f64, 30>::new(0.0);
        loop {
            pps.wait_for_pps().await;
            let pps_time = clock.now_ms();
            let gps_location = gps_parser.get_nmea();
            if let Some(gps_timestamp) = gps_location.gps_timestamp {
                let current_unix_timestamp = ((gps_timestamp + 1) as f64) * 1000.0;
                let new_offset = current_unix_timestamp - pps_time;
                // log_info!("New offset: {}", new_offset);
                offset_running_avg.add_sample(new_offset);
                self.state.lock(|state| {
                    let mut state = state.borrow_mut();
                    state.offset.replace(offset_running_avg.get_average());
                    state.waker.wake();
                });
            }
        }
    }
}

#[derive(Clone)]
pub struct UnixClock<'a, K: Clock> {
    task: &'a UnixClockTask<K>,
}

impl<'a, K: Clock> UnixClock<'a, K> {
    fn new(task: &'a UnixClockTask<K>) -> Self {
        Self { task }
    }

    pub fn ready(&self) -> bool {
        self.task.state.lock(|state| state.borrow().offset.is_some())
    }

    pub async fn wait_until_ready(&self) {
        poll_fn(|cx| {
            self.task.state.lock(|state|{
                let mut state = state.borrow_mut();
                if state.offset.is_some() {
                    return Poll::Ready(());
                } else {
                    state.waker.register(cx.waker());
                    return Poll::Pending;
                }
            })
        }).await;
    }

    pub fn convert_to_unix(&self, boot_timstamp: f64) -> f64 {
        let offset = self.task.state.lock(|state| {
            let state = state.borrow();
            let offset = state.offset.unwrap_or(0.0);
            offset
        });
        boot_timstamp + offset
    }
}

impl<'a, K: Clock> Clock for UnixClock<'a, K> {
    // not guaranteed to be monotonically increasing (probably is)
    fn now_ms(&self) -> f64 {
        self.task.state.lock(|state| {
            let state = state.borrow();
            let offset = state.offset.unwrap_or(0.0);
            self.task.clock.now_ms() + offset
        })
    }
}
