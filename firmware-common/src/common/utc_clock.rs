use core::cell::RefCell;

use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};

use crate::{driver::gps::GPSPPS, Clock};

use super::gps_parser::GPSParser;

pub struct UtcClockTask<K: Clock> {
    offset: BlockingMutex<NoopRawMutex, RefCell<Option<f64>>>,
    clock: K,
}

impl<K: Clock> UtcClockTask<K> {
    pub fn new(clock: K) -> Self {
        Self {
            offset: BlockingMutex::new(RefCell::new(None)),
            clock,
        }
    }

    pub fn get_clock(&self) -> UtcClock<K> {
        UtcClock::new(self)
    }

    pub async fn run(&self, mut pps: impl GPSPPS, gps_parser: &GPSParser, clock: impl Clock) {
        loop {
            pps.wait_for_pps().await;
            let pps_time = clock.now_ms();
            let gps_location = gps_parser.get_nmea();
            if let Some(gps_timestamp) = gps_location.gps_timestamp {
                let current_timestamp_ms = ((gps_timestamp + 1) as f64) * 1000.0;
                self.offset
                    .lock(|offset| offset.borrow_mut().replace(current_timestamp_ms - pps_time));
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct UtcClock<'a, K: Clock> {
    task: &'a UtcClockTask<K>,
}

impl<'a, K: Clock> UtcClock<'a, K> {
    fn new(task: &'a UtcClockTask<K>) -> Self {
        Self { task }
    }

    pub fn ready(&self) -> bool {
        self.task.offset.lock(|offset| offset.borrow().is_some())
    }
}

impl<'a, K: Clock> Clock for UtcClock<'a, K> {
    // not guaranteed to be monotonically increasing
    fn now_ms(&self) -> f64 {
        self.task.offset.lock(|offset| {
            let offset = offset.borrow();
            let offset = offset.expect("UTC clock not ready");
            self.task.clock.now_ms() + offset
        })
    }
}
