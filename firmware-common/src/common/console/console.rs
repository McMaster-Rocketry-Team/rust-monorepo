use core::{cell::RefCell, future::poll_fn, task::Poll};
use embassy_futures::yield_now;

use crate::{common::multi_waker::MultiWakerRegistration, driver::serial::Serial};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    blocking_mutex::Mutex as BlockingMutex,
    mutex::{Mutex, MutexGuard},
};
use heapless::Vec;

struct ConsoleState<const N: usize> {
    current_command_id: Option<u64>,
    wakers_reg: MultiWakerRegistration<N>,
    command_ids_listening: Vec<u64, N>,
}

pub struct Console<S: Serial, const N: usize> {
    serial: Mutex<NoopRawMutex, S>,
    state: BlockingMutex<NoopRawMutex, RefCell<ConsoleState<N>>>,
}

impl<S: Serial, const N: usize> Console<S, N> {
    pub fn new(serial: S) -> Self {
        Self {
            serial: Mutex::new(serial),
            state: BlockingMutex::new(RefCell::new(ConsoleState {
                current_command_id: None,
                wakers_reg: MultiWakerRegistration::new(),
                command_ids_listening: Vec::new(),
            })),
        }
    }

    pub async fn wait_for_command(&self, waiting_command_id: u64) -> ConsoleSerialGuard<S, N> {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            state
                .command_ids_listening
                .push(waiting_command_id)
                .unwrap(); // TODO handle already listening
        });
        poll_fn(|ctx| {
            self.state.lock(|state| {
                let mut state = state.borrow_mut();
                if state.current_command_id == Some(waiting_command_id) {
                    state.current_command_id = None;
                    let index = state
                        .command_ids_listening
                        .iter()
                        .position(|id| *id == waiting_command_id)
                        .unwrap();
                    state.command_ids_listening.swap_remove(index);
                    Poll::Ready(ConsoleSerialGuard::new(&self))
                } else {
                    state.wakers_reg.register(ctx.waker()).unwrap(); // FIXME unwrap
                    Poll::Pending
                }
            })
        })
        .await
    }

    pub async fn run_dispatcher(&self) -> ! {
        let mut command_buffer = [0u8; 8];
        loop {
            let mut serial = self.serial.lock().await;
            if serial.read_all(&mut command_buffer).await.is_err() {
                continue;
            };
            drop(serial);
            let command_id = u64::from_be_bytes(command_buffer);
            log_info!("Received command: {:x}", command_id);

            let has_listener = self.state.lock(|state| {
                let mut state = state.borrow_mut();
                let has_listener = state
                    .command_ids_listening
                    .iter()
                    .position(|id| *id == command_id)
                    .is_some();
                if has_listener {
                    state.current_command_id = Some(command_id);
                    state.wakers_reg.wake();
                }
                has_listener
            });

            if has_listener {
                yield_now().await;
                while self
                    .state
                    .lock(|state| state.borrow().current_command_id != None)
                {
                    yield_now().await;
                }
            }
        }
    }
}

pub struct ConsoleSerialGuard<'b, S: Serial, const N: usize> {
    serial_mutex_guard: MutexGuard<'b, NoopRawMutex, S>,
}

impl<'b, S: Serial, const N: usize> ConsoleSerialGuard<'b, S, N> {
    fn new<'a: 'b>(console: &'a Console<S, N>) -> Self {
        let serial_mutex_guard = console.serial.try_lock().unwrap();
        Self { serial_mutex_guard }
    }
}

impl<'b, S: Serial, const N: usize> Serial for ConsoleSerialGuard<'b, S, N> {
    type Error = S::Error;

    async fn write(&mut self, data: &[u8]) -> Result<(), S::Error> {
        self.serial_mutex_guard.write(data).await
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, S::Error> {
        self.serial_mutex_guard.read(buffer).await
    }
}
