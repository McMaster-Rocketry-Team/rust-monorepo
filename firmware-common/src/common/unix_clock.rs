use core::{cell::RefCell, future::poll_fn, task::Poll};

use embassy_sync::{
    blocking_mutex::{
        raw::{NoopRawMutex, RawMutex},
        Mutex as BlockingMutex,
    },
    pubsub::Subscriber,
};
use futures::join;

use crate::{driver::gps::GPSPPS, Clock};

use super::{moving_average::NoSumSMA, multi_waker::MultiWakerRegistration};

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

    pub async fn run<const CAP: usize, const SUBS: usize, const PUBS: usize>(
        &self,
        mut pps: impl GPSPPS,
        mut gps_timestamp_sub: Subscriber<'_, impl RawMutex, i64, CAP, SUBS, PUBS>,
        clock: impl Clock,
    ) -> ! {
        let mut offset_running_avg = NoSumSMA::<f64, f64, 30>::new(0.0);
        let latest_gps_timestamp =
            BlockingMutex::<NoopRawMutex, _>::new(RefCell::new((0f64, 0i64)));

        let receive_gps_timestamp_fut = async {
            loop {
                let gps_timestamp = gps_timestamp_sub.next_message_pure().await;
                latest_gps_timestamp.lock(|latest_gps_timestamp| {
                    *latest_gps_timestamp.borrow_mut() = (clock.now_ms(), gps_timestamp);
                });
            }
        };

        let wait_for_pps_fut = async {
            loop {
                pps.wait_for_pps().await;
                let pps_time = clock.now_ms();
                latest_gps_timestamp.lock(|latest_gps_timestamp| {
                    let latest_gps_timestamp = latest_gps_timestamp.borrow();
                    if pps_time - latest_gps_timestamp.0 < 800.0 {
                        let current_unix_timestamp = ((latest_gps_timestamp.1 + 1) as f64) * 1000.0;
                        let new_offset = current_unix_timestamp - pps_time;
                        offset_running_avg.add_sample(new_offset);
                        self.state.lock(|state| {
                            let mut state = state.borrow_mut();
                            state.offset.replace(offset_running_avg.get_average());
                            state.waker.wake();
                        });
                    }
                });
            }
        };

        join!(receive_gps_timestamp_fut, wait_for_pps_fut);
        log_unreachable!()
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
        self.task
            .state
            .lock(|state| state.borrow().offset.is_some())
    }

    pub async fn wait_until_ready(&self) {
        poll_fn(|cx| {
            self.task.state.lock(|state| {
                let mut state = state.borrow_mut();
                if state.offset.is_some() {
                    return Poll::Ready(());
                } else {
                    state.waker.register(cx.waker());
                    return Poll::Pending;
                }
            })
        })
        .await;
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
