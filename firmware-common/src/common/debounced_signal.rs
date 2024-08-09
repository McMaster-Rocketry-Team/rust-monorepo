use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    signal::Signal,
};

use crate::Delay;

pub struct DebouncedSignal<R, T>
where
    R: RawMutex,
    T: Clone + PartialEq + Send,
{
    changed_signal: Signal<R, T>,
    debounced_signal: Signal<R, T>,
    debounce_ms: f64,
}

impl<R, T> DebouncedSignal<R, T>
where
    R: RawMutex,
    T: Clone + PartialEq + Send,
{
    pub fn new(debounce_ms: f64) -> Self {
        Self {
            changed_signal: Signal::new(),
            debounced_signal: Signal::new(),
            debounce_ms,
        }
    }

    pub fn signal(&self, value: T) {
        self.changed_signal.signal(value);
    }

    pub async fn wait(&self) -> T {
        self.debounced_signal.wait().await
    }

    pub async fn run_debounce(&self, delay: impl Delay) {
        let mut state = self.changed_signal.wait().await;
        self.debounced_signal.signal(state.clone());

        let mut new_state = self.changed_signal.wait().await;
        loop {
            let fut_1 = delay.delay_ms(self.debounce_ms);
            let fut_2 = self.changed_signal.wait();

            match select(fut_1, fut_2).await {
                Either::First(_) => {
                    if new_state != state {
                        state = new_state.clone();
                        self.debounced_signal.signal(state.clone());
                    }
                }
                Either::Second(new_new_state) => {
                    new_state = new_new_state;
                }
            };
        }
    }
}
