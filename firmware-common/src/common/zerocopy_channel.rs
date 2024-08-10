use core::ops::{Deref, DerefMut};

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    mutex::{Mutex, MutexGuard},
    signal::Signal,
};

#[derive(Debug, Clone, Copy, defmt::Format)]
enum ReadyValueSelect {
    One,
    Two,
}

impl ReadyValueSelect {
    fn next(&self) -> Self {
        match self {
            ReadyValueSelect::One => ReadyValueSelect::Two,
            ReadyValueSelect::Two => ReadyValueSelect::One,
        }
    }
}

pub struct ZeroCopyChannel<M: RawMutex, T> {
    ready_value_signal: Signal<M, ReadyValueSelect>,
    value_1: Mutex<M, T>,
    value_2: Mutex<M, T>,
}

impl<M: RawMutex, T> ZeroCopyChannel<M, T> {
    pub const fn new(value_1: T, value_2: T) -> Self {
        Self {
            ready_value_signal: Signal::new(),
            value_1: Mutex::new(value_1),
            value_2: Mutex::new(value_2),
        }
    }

    /// Warning: only one sender per channel is supported
    pub fn sender(&self) -> ZeroCopyChannelSender<M, T> {
        ZeroCopyChannelSender {
            channel: self,
            curr_ready_value: ReadyValueSelect::One,
        }
    }

    pub fn receiver(&self) -> ZeroCopyChannelReceiver<M, T> {
        ZeroCopyChannelReceiver { channel: self }
    }
}

pub struct ZeroCopyChannelSender<'a, M: RawMutex, T> {
    channel: &'a ZeroCopyChannel<M, T>,
    curr_ready_value: ReadyValueSelect,
}

impl<'a, M: RawMutex, T> ZeroCopyChannelSender<'a, M, T> {
    pub fn try_start_send(&mut self) -> Option<ZeroCopyChannelSendGuard<'a, '_, M, T>> {
        let result = match self.curr_ready_value {
            ReadyValueSelect::One => self.channel.value_1.try_lock(),
            ReadyValueSelect::Two => self.channel.value_2.try_lock(),
        };

        match result {
            Ok(guard) => Some(ZeroCopyChannelSendGuard {
                sender: self,
                guard: Some(guard),
            }),
            Err(_) => None,
        }
    }

    pub async fn start_send(&mut self) -> ZeroCopyChannelSendGuard<'a, '_, M, T> {
        let guard = match self.curr_ready_value {
            ReadyValueSelect::One => self.channel.value_1.lock().await,
            ReadyValueSelect::Two => self.channel.value_2.lock().await,
        };

        ZeroCopyChannelSendGuard {
            sender: self,
            guard: Some(guard),
        }
    }
}

pub struct ZeroCopyChannelSendGuard<'a, 'b, M: RawMutex, T> {
    sender: &'b mut ZeroCopyChannelSender<'a, M, T>,
    guard: Option<MutexGuard<'a, M, T>>,
}

impl<'a, 'b, M: RawMutex, T> Deref for ZeroCopyChannelSendGuard<'a, 'b, M, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap().deref()
    }
}

impl<'a, 'b, M: RawMutex, T> DerefMut for ZeroCopyChannelSendGuard<'a, 'b, M, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}

impl<'a, 'b, M: RawMutex, T> Drop for ZeroCopyChannelSendGuard<'a, 'b, M, T> {
    fn drop(&mut self) {
        drop(self.guard.take());
        self.sender
            .channel
            .ready_value_signal
            .signal(self.sender.curr_ready_value);
        self.sender.curr_ready_value = self.sender.curr_ready_value.next();
    }
}

pub struct ZeroCopyChannelReceiver<'a, M: RawMutex, T> {
    channel: &'a ZeroCopyChannel<M, T>,
}

impl<'a, M: RawMutex, T> ZeroCopyChannelReceiver<'a, M, T> {
    pub async fn receive(&self) -> MutexGuard<'a, M, T> {
        let ready_value = self.channel.ready_value_signal.wait().await;
        let guard = match ready_value {
            ReadyValueSelect::One => self.channel.value_1.lock().await,
            ReadyValueSelect::Two => self.channel.value_2.lock().await,
        };

        guard
    }
}
