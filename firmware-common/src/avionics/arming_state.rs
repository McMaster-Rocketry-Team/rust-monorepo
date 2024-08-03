use core::cell::RefCell;

use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::{raw::RawMutex, Mutex as BlockingMutex},
    signal::Signal,
};

use super::Delay;

struct ArmingStateInternal {
    software_armed: bool,
    hardware_armed: bool,
}

impl ArmingStateInternal {
    fn new() -> Self {
        Self {
            software_armed: false,
            hardware_armed: false,
        }
    }

    fn armed(&self) -> bool {
        self.software_armed && self.hardware_armed
    }
}

pub struct ArmingState<R: RawMutex> {
    state: BlockingMutex<R, RefCell<ArmingStateInternal>>,
    arming_changed_signal: Signal<R, bool>,
    debounced_arming_changed_signal: Signal<R, bool>,
}

impl<R: RawMutex> ArmingState<R> {
    pub fn new() -> Self {
        Self {
            state: BlockingMutex::new(RefCell::new(ArmingStateInternal::new())),
            arming_changed_signal: Signal::new(),
            debounced_arming_changed_signal: Signal::new(),
        }
    }

    pub fn set_software_armed(&self, armed: bool) {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.software_armed = armed;
            self.arming_changed_signal.signal(s.armed());
        });
    }

    pub fn set_hardware_armed(&self, armed: bool) {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.hardware_armed = armed;
            self.arming_changed_signal.signal(s.armed());
        });
    }

    pub async fn wait(&self) -> bool {
        self.debounced_arming_changed_signal.wait().await
    }

    pub fn is_armed(&self) -> bool {
        self.state.lock(|s| s.borrow().armed())
    }

    pub async fn run_debounce(&self, delay: impl Delay) {
        let mut armed = self.is_armed();
        self.debounced_arming_changed_signal.signal(armed);
        let new_armed =
            BlockingMutex::<R, _>::new(RefCell::new(self.arming_changed_signal.wait().await));
        loop {
            let fut_1 = async {
                delay.delay_ms(1000.0).await;
                let new_armed = new_armed.lock(|s| *s.borrow());
                if new_armed != armed {
                    armed = new_armed;
                    self.debounced_arming_changed_signal.signal(armed);
                }
            };

            let fut_2 = async {
                let signal = self.arming_changed_signal.wait().await;
                new_armed.lock(|s| s.replace(signal));
            };

            select(fut_1, fut_2).await;
        }
    }
}
