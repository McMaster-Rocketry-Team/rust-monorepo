use core::cell::RefCell;

use embassy_sync::{
    blocking_mutex::{raw::RawMutex, Mutex as BlockingMutex},
    pubsub::{PubSubBehavior, PubSubChannel, Subscriber},
};
use futures::join;

use crate::common::debounced_signal::DebouncedSignal;

use super::Delay;

#[derive(Debug, Clone)]
pub struct ArmingState {
    pub software_armed: bool,
    pub hardware_armed: bool,
}

impl ArmingState {
    fn new() -> Self {
        Self {
            software_armed: false,
            hardware_armed: false,
        }
    }

    pub fn is_armed(&self) -> bool {
        self.software_armed && self.hardware_armed
    }
}

pub struct ArmingStateManager<R: RawMutex> {
    state: BlockingMutex<R, RefCell<ArmingState>>,
    pub_sub: PubSubChannel<R, ArmingState, 1, 5, 1>,
    hardware_armed_debounced_signal: DebouncedSignal<R, bool>,
}

impl<R: RawMutex> ArmingStateManager<R> {
    pub fn new() -> Self {
        Self {
            state: BlockingMutex::new(RefCell::new(ArmingState::new())),
            pub_sub: PubSubChannel::new(),
            hardware_armed_debounced_signal: DebouncedSignal::new(1000.0),
        }
    }

    pub fn set_software_armed(&self, armed: bool) {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.software_armed = armed;
            self.pub_sub.publish_immediate(s.clone());
        });
    }

    pub fn set_hardware_armed(&self, armed: bool) {
        self.hardware_armed_debounced_signal.signal(armed);
    }

    pub fn is_armed(&self) -> bool {
        self.state.lock(|s| s.borrow().is_armed())
    }

    pub fn subscriber(&self) -> Subscriber<R, ArmingState, 1, 5, 1> {
        self.pub_sub.subscriber().unwrap()
    }

    pub async fn run_debounce(&self, delay: impl Delay) {
        let debounce_fut = self.hardware_armed_debounced_signal.run_debounce(delay);

        let set_hardware_armed_fut = async {
            loop {
                let hardware_armed = self.hardware_armed_debounced_signal.wait().await;
                self.state.lock(|s| {
                    let mut s = s.borrow_mut();
                    s.hardware_armed = hardware_armed;
                    self.pub_sub.publish_immediate(s.clone());
                });
            }
        };

        join!(debounce_fut, set_hardware_armed_fut);
    }
}
