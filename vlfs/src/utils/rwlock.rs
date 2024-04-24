use core::cell::{RefCell, UnsafeCell};
use core::future::poll_fn;
use core::ops::{Deref, DerefMut};
use core::task::Poll;

use defmt::Format;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::{blocking_mutex::raw::RawMutex, waitqueue::MultiWakerRegistration};

#[derive(Format)]
enum LockedState {
    Unlocked,
    ReadLocked(usize),
    WriteLocked,
}

struct State<const N: usize> {
    locked: LockedState,
    writer_pending: usize,
    waker: MultiWakerRegistration<N>,
}

pub struct RwLock<M, T, const N: usize>
where
    M: RawMutex,
    T: ?Sized,
{
    state: BlockingMutex<M, RefCell<State<N>>>,
    inner: UnsafeCell<T>,
}

impl<M, T, const N: usize> RwLock<M, T, N>
where
    M: RawMutex,
{
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
            state: BlockingMutex::new(RefCell::new(State {
                locked: LockedState::Unlocked,
                writer_pending: 0,
                waker: MultiWakerRegistration::new(),
            })),
        }
    }
}

impl<M, T, const N: usize> RwLock<M, T, N>
where
    M: RawMutex,
{
    pub async fn read(&self) -> RwLockReadGuard<'_, M, T, N> {
        poll_fn(|cx| {
            let ready = self.state.lock(|s| {
                let mut s = s.borrow_mut();
                if s.writer_pending > 0 {
                    return false;
                }
                match s.locked {
                    LockedState::Unlocked => {
                        s.locked = LockedState::ReadLocked(1);
                        true
                    }
                    LockedState::ReadLocked(n) => {
                        s.locked = LockedState::ReadLocked(n + 1);
                        true
                    }
                    LockedState::WriteLocked => {
                        s.waker.register(cx.waker()); // TODO could go wrong?
                        false
                    }
                }
            });

            if ready {
                Poll::Ready(RwLockReadGuard { rwlock: self })
            } else {
                Poll::Pending
            }
        })
        .await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, M, T, N> {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.writer_pending += 1;
        });
        poll_fn(|cx| {
            let ready = self.state.lock(|s| {
                let mut s = s.borrow_mut();
                match s.locked {
                    LockedState::Unlocked => {
                        s.writer_pending -= 1;
                        s.locked = LockedState::WriteLocked;
                        true
                    }
                    _ => {
                        s.waker.register(cx.waker()); // TODO could go wrong?
                        false
                    }
                }
            });

            if ready {
                Poll::Ready(RwLockWriteGuard { rwlock: self })
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

pub struct RwLockReadGuard<'a, M, T, const N: usize>
where
    M: RawMutex,
    T: ?Sized,
{
    rwlock: &'a RwLock<M, T, N>,
}

impl<'a, M, T, const N: usize> Drop for RwLockReadGuard<'a, M, T, N>
where
    M: RawMutex,
    T: ?Sized,
{
    fn drop(&mut self) {
        self.rwlock.state.lock(|s| {
            let mut s = s.borrow_mut();
            match s.locked {
                LockedState::ReadLocked(n) => {
                    if n == 1 {
                        s.locked = LockedState::Unlocked;
                    } else {
                        s.locked = LockedState::ReadLocked(n - 1);
                    }
                }
                _ => panic!("invalid state"),
            };
            s.waker.wake();
        });
    }
}

impl<'a, M, T, const N: usize> Deref for RwLockReadGuard<'a, M, T, N>
where
    M: RawMutex,
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.rwlock.inner.get() as *const T) }
    }
}

pub struct RwLockWriteGuard<'a, M, T, const N: usize>
where
    M: RawMutex,
    T: ?Sized,
{
    rwlock: &'a RwLock<M, T, N>,
}

impl<'a, M, T, const N: usize> Drop for RwLockWriteGuard<'a, M, T, N>
where
    M: RawMutex,
    T: ?Sized,
{
    fn drop(&mut self) {
        self.rwlock.state.lock(|s| {
            let mut s = s.borrow_mut();
            match s.locked {
                LockedState::WriteLocked => {
                    s.locked = LockedState::Unlocked;
                }
                _ => panic!("invalid state"),
            };
            s.waker.wake();
        });
    }
}

impl<'a, M, T, const N: usize> Deref for RwLockWriteGuard<'a, M, T, N>
where
    M: RawMutex,
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.rwlock.inner.get() as *const T) }
    }
}

impl<'a, M, T, const N: usize> DerefMut for RwLockWriteGuard<'a, M, T, N>
where
    M: RawMutex,
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.rwlock.inner.get()) }
    }
}

// pub async fn rwLockTest<T: Timer>(timer: &T) {
//     async {
//         info!("rw lock test 1 started! =====");
//         let data = RwLock::<NoopRawMutex, _, 10>::new(1);

//         let task1 = async {
//             let read1 = data.read().await;
//             info!("task1: start {}", *read1);
//             timer.sleep(100).await;
//             info!("task1: end {}", *read1);
//         };

//         let task2 = async {
//             let read2 = data.read().await;
//             info!("task2: start {}", *read2);
//             timer.sleep(100).await;
//             info!("task2: end {}", *read2);
//         };

//         let task3 = async {
//             timer.sleep(50).await;
//             let mut write = data.write().await;
//             info!("task3: start {}", *write);
//             *write = 2;
//             timer.sleep(100).await;
//             info!("task3: end {}", *write);
//         };

//         let task4 = async {
//             timer.sleep(125).await;
//             let read4 = data.read().await;
//             info!("task4: start {}", *read4);
//             timer.sleep(100).await;
//             info!("task4: end {}", *read4);
//         };

//         join(join(task1, task2), join(task3, task4)).await;
//     }
//     .await;

//     async {
//         info!("rw lock test 2 started! =====");
//         let data = RwLock::<NoopRawMutex, _, 10>::new(1);

//         let task1 = async {
//             loop {
//                 let read1 = data.read().await;
//                 info!("task1: start {}", *read1);
//                 timer.sleep(100).await;
//                 info!("task1: end {}", *read1);
//                 if *read1 == 2 {
//                     break;
//                 }
//             }
//         };

//         let task2 = async {
//             timer.sleep(50).await;

//             loop {
//                 let read2 = data.read().await;
//                 info!("task2: start {}", *read2);
//                 timer.sleep(100).await;
//                 info!("task2: end {}", *read2);
//                 if *read2 == 2 {
//                     break;
//                 }
//             }
//         };

//         let task3 = async {
//             timer.sleep(125).await;

//             let mut write3 = data.write().await;
//             info!("task3: start {}", *write3);
//             *write3 = 2;
//             timer.sleep(100).await;
//             info!("task3: end {}", *write3);
//         };

//         join(join(task1, task2), task3).await;
//     }
//     .await;
// }
