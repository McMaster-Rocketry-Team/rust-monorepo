use core::{
    array,
    ops::{Deref, DerefMut},
};

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    mutex::{Mutex, MutexGuard},
    signal::Signal,
};

pub struct ZeroCopyChannel<M: RawMutex, T: Default, const N: usize> {
    ready_value_signal: Signal<M, usize>,
    values: [Mutex<M, T>; N],
}

impl<M: RawMutex, T: Default, const N: usize> ZeroCopyChannel<M, T, N> {
    pub fn new() -> Self {
        Self {
            ready_value_signal: Signal::new(),
            values: array::from_fn(|_| Mutex::new(T::default())),
        }
    }

    /// Warning: only one sender per channel is supported
    pub fn sender(&self) -> ZeroCopyChannelSender<M, T, N> {
        ZeroCopyChannelSender {
            channel: self,
            curr_ready_value: 0,
        }
    }

    pub fn receiver(&self) -> ZeroCopyChannelReceiver<M, T, N> {
        ZeroCopyChannelReceiver { channel: self }
    }
}

pub struct ZeroCopyChannelSender<'a, M: RawMutex, T: Default, const N: usize> {
    channel: &'a ZeroCopyChannel<M, T, N>,
    curr_ready_value: usize,
}

impl<'a, M: RawMutex, T: Default, const N: usize> ZeroCopyChannelSender<'a, M, T, N> {
    pub fn try_start_send(&mut self) -> Option<ZeroCopyChannelSendGuard<'a, '_, M, T, N>> {
        let result = self.channel.values[self.curr_ready_value].try_lock();

        match result {
            Ok(guard) => Some(ZeroCopyChannelSendGuard {
                sender: self,
                guard: Some(guard),
            }),
            Err(_) => None,
        }
    }

    pub async fn start_send(&mut self) -> ZeroCopyChannelSendGuard<'a, '_, M, T, N> {
        let guard = self.channel.values[self.curr_ready_value].lock().await;

        ZeroCopyChannelSendGuard {
            sender: self,
            guard: Some(guard),
        }
    }
}

pub struct ZeroCopyChannelSendGuard<'a, 'b, M: RawMutex, T: Default, const N: usize> {
    sender: &'b mut ZeroCopyChannelSender<'a, M, T, N>,
    guard: Option<MutexGuard<'a, M, T>>,
}

impl<'a, 'b, M: RawMutex, T: Default, const N: usize> Deref
    for ZeroCopyChannelSendGuard<'a, 'b, M, T, N>
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap().deref()
    }
}

impl<'a, 'b, M: RawMutex, T: Default, const N: usize> DerefMut
    for ZeroCopyChannelSendGuard<'a, 'b, M, T, N>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}

impl<'a, 'b, M: RawMutex, T: Default, const N: usize> Drop
    for ZeroCopyChannelSendGuard<'a, 'b, M, T, N>
{
    fn drop(&mut self) {
        drop(self.guard.take());
        self.sender
            .channel
            .ready_value_signal
            .signal(self.sender.curr_ready_value);
        self.sender.curr_ready_value += 1;
        if self.sender.curr_ready_value >= N {
            self.sender.curr_ready_value = 0;
        }
    }
}

pub struct ZeroCopyChannelReceiver<'a, M: RawMutex, T: Default, const N: usize> {
    channel: &'a ZeroCopyChannel<M, T, N>,
}

impl<'a, M: RawMutex, T: Default, const N: usize> ZeroCopyChannelReceiver<'a, M, T, N> {
    pub async fn receive(&self) -> MutexGuard<'a, M, T> {
        let ready_value = self.channel.ready_value_signal.wait().await;
        let guard = self.channel.values[ready_value].lock().await;

        guard
    }
}
