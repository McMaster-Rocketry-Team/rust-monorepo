use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    future::poll_fn,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};
use firmware_common::driver::delay::Delay as DelayDriver;
use firmware_common::driver::clock::Clock as ClockDriver;
use embedded_hal_async::delay::DelayNs;

pub struct SimulationClock {
    time: f64, // in seconds
    wakers: BinaryHeap<Reverse<DelayWaker>>,
}

impl SimulationClock {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            wakers: BinaryHeap::new(),
        }
    }

    pub fn register_waker(&mut self, waker: Waker, wake_time: f64) {
        self.wakers.push(Reverse(DelayWaker { wake_time, waker }));
    }

    pub fn update_time(&mut self, time: f64) {
        self.time = time;
        while let Some(Reverse(waker)) = self.wakers.peek() {
            if waker.wake_time <= self.time {
                waker.waker.wake_by_ref();
                self.wakers.pop();
            } else {
                break;
            }
        }
    }
}

struct DelayWaker {
    wake_time: f64,
    waker: Waker,
}

impl PartialEq for DelayWaker {
    fn eq(&self, other: &Self) -> bool {
        self.wake_time == other.wake_time
    }
}

impl Eq for DelayWaker {}

impl PartialOrd for DelayWaker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.wake_time.partial_cmp(&other.wake_time)
    }
}

impl Ord for DelayWaker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.wake_time.partial_cmp(&other.wake_time).unwrap()
    }
}

#[derive(Clone)]
pub struct Delay {
    clock: Arc<Mutex<SimulationClock>>,
}

impl Delay {
    pub fn new(clock: Arc<Mutex<SimulationClock>>) -> Self {
        Self { clock }
    }
}

impl DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        if ns == 0 {
            return;
        }
        let wake_time = {
            let clock = self.clock.lock().unwrap();
            clock.time + ns as f64 * 1e-9
        };
        poll_fn(|cx| {
            let mut clock = self.clock.lock().unwrap();

            if wake_time <= clock.time {
                return Poll::Ready(());
            } else {
                clock.register_waker(cx.waker().clone(), wake_time);
                return Poll::Pending;
            }
        })
        .await;
    }
}

impl DelayDriver for Delay {
    async fn delay_ms(&self, ms: f64) {
        if ms <= 0.0 {
            return;
        }
        let wake_time = {
            let clock = self.clock.lock().unwrap();
            clock.time + ms * 1e-3
        };
        poll_fn(|cx| {
            let mut clock = self.clock.lock().unwrap();

            if wake_time <= clock.time {
                return Poll::Ready(());
            } else {
                clock.register_waker(cx.waker().clone(), wake_time);
                return Poll::Pending;
            }
        })
        .await;
    }
}

#[derive(Clone)]
pub struct Clock {
    clock: Arc<Mutex<SimulationClock>>,
}

impl Clock {
    pub fn new(clock: Arc<Mutex<SimulationClock>>) -> Self {
        Self { clock }
    }
}

impl ClockDriver for Clock {
    fn now_ms(&self) -> f64 {
        self.clock.lock().unwrap().time * 1e3
    }
}