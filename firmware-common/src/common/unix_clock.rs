use core::cell::RefCell;

use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};

use crate::{driver::gps::GPSPPS, Clock};

use super::{gps_parser::GPSParser, moving_average::NoSumSMA};

pub struct UnixClockTask<K: Clock> {
    offset: BlockingMutex<NoopRawMutex, RefCell<Option<f64>>>,
    clock: K,
}

impl<K: Clock> UnixClockTask<K> {
    pub fn new(clock: K) -> Self {
        Self {
            offset: BlockingMutex::new(RefCell::new(None)),
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
                offset_running_avg.add_sample(new_offset);
                self.offset
                    .lock(|offset| offset.borrow_mut().replace(offset_running_avg.get_average()));
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct UnixClock<'a, K: Clock> {
    task: &'a UnixClockTask<K>,
}

impl<'a, K: Clock> UnixClock<'a, K> {
    fn new(task: &'a UnixClockTask<K>) -> Self {
        Self { task }
    }

    pub fn ready(&self) -> bool {
        self.task.offset.lock(|offset| offset.borrow().is_some())
    }

    pub fn convert_to_unix(&self, boot_timstamp: f64) -> f64 {
        let offset = self.task.offset.lock(|offset| {
            let offset = offset.borrow();
            let offset = offset.unwrap_or(0.0);
            offset
        });
        boot_timstamp + offset
    }
}

impl<'a, K: Clock> Clock for UnixClock<'a, K> {
    // not guaranteed to be monotonically increasing (probably is)
    fn now_ms(&self) -> f64 {
        self.task.offset.lock(|offset| {
            let offset = offset.borrow();
            let offset = offset.unwrap_or(0.0);
            self.task.clock.now_ms() + offset
        })
    }
}
