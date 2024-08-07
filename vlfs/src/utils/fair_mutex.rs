use core::cell::{RefCell, UnsafeCell};
use core::ops::{Deref, DerefMut};
use core::task::{Poll, Waker};

use core::future::poll_fn;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;

use heapless::Deque;

struct State<const N: usize> {
    locked: bool,
    waker_queue: Deque<Waker, N>,
}

pub struct FairMutex<M, T, const N: usize>
where
    M: RawMutex,
{
    state: BlockingMutex<M, RefCell<State<N>>>,
    inner: UnsafeCell<T>,
}

impl<M, T, const N: usize> FairMutex<M, T, N>
where
    M: RawMutex,
{
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
            state: BlockingMutex::new(RefCell::new(State {
                locked: false,
                waker_queue: Deque::new(),
            })),
        }
    }
}

impl<M, T, const N: usize> FairMutex<M, T, N>
where
    M: RawMutex,
{
    pub async fn lock(&self) -> FairMutexGuard<'_, M, T, N> {
        poll_fn(|cx| {
            let ready = self.state.lock(|s| {
                let mut s = s.borrow_mut();
                if s.locked {
                    s.waker_queue.push_back(cx.waker().clone()).ok();
                    return false;
                }
                s.locked = true;
                true
            });
            if ready {
                Poll::Ready(FairMutexGuard { mutex: self })
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

pub struct FairMutexGuard<'a, M, T, const N: usize>
where
    M: RawMutex,
{
    mutex: &'a FairMutex<M, T, N>,
}

impl<'a, M, T, const N: usize> Drop for FairMutexGuard<'a, M, T, N>
where
    M: RawMutex,
{
    fn drop(&mut self) {
        self.mutex.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.locked = false;
            if let Some(waker) = s.waker_queue.pop_front() {
                waker.wake();
            }
        });
    }
}

impl<'a, M, T, const N: usize> Deref for FairMutexGuard<'a, M, T, N>
where
    M: RawMutex,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<'a, M, T, const N: usize> DerefMut for FairMutexGuard<'a, M, T, N>
where
    M: RawMutex,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.inner.get() }
    }
}



#[cfg(test)]
mod tests {
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use tokio::join;
    use super::*;

    #[tokio::test]
    async fn fair_mutex_test() {
        let m: FairMutex<NoopRawMutex, u32, 10> = FairMutex::new(0);
        let task1 = async {
            loop {
                log_info!("Task 1");
                let mut guard = m.lock().await;
                *guard += 1;
                if *guard > 100 {
                    break;
                }
            }
        };
        let task2 = async {
            loop {
                log_info!("Task 2");
                let mut guard = m.lock().await;
                *guard += 1;
                if *guard > 100 {
                    break;
                }
            }
        };
    
        join!(task1, task2);
    }
}
