use core::cell::RefCell;

use defmt::warn;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use firmware_common::driver::serial::Serial;
use heapless::Deque;

use crate::codec::{decode_package, DecodedPackage};
use crate::packages::ack::Ack;
use crate::packages::device::{DeviceInfo, GetDevice};
use crate::packages::event::Event;
use crate::packages::pyro::PyroCtrl;

pub struct Slave<S: Serial, A, B>
where
    A: Fn(GetDevice) -> DeviceInfo,
    B: Fn(PyroCtrl) -> Ack,
{
    events: BlockingMutex<CriticalSectionRawMutex, RefCell<Deque<Event, 5>>>,
    serial: Mutex<CriticalSectionRawMutex, S>,
    on_get_device: A,
    on_pyro_ctrl: B,
}

impl<S: Serial, A, B> Slave<S, A, B>
where
    A: Fn(GetDevice) -> DeviceInfo,
    B: Fn(PyroCtrl) -> Ack,
{
    pub fn new(serial: S, on_get_device: A, on_pyro_ctrl: B) -> Self {
        Self {
            events: BlockingMutex::new(RefCell::new(Deque::new())),
            serial: Mutex::new(serial),
            on_get_device,
            on_pyro_ctrl,
        }
    }

    pub fn push_event(&self, event: Event) {
        self.events.lock(|events| {
            let mut events = events.borrow_mut();

            if events.is_full() {
                events.pop_front();
                warn!("Event queue is full, dropping oldest event");
            }
            events.push_back(event).unwrap();
        });
    }

    pub async fn run(&self) -> ! {
        let mut buffer = [0u8; 128];
        loop {
            let _: Result<(), S::Error> = try {
                let mut serial = self.serial.lock().await;
                let len = serial.read(&mut buffer).await?;
                let package = decode_package(&buffer[..len]);
                match package {
                    Ok(package) => match package {
                        DecodedPackage::GetDevice(get_device) => {
                            let device_info = (self.on_get_device)(get_device);
                            let encoded = crate::codec::encode_package(&mut buffer, device_info);
                            serial.write(encoded).await?;
                        }
                        DecodedPackage::PyroCtrl(pyro_ctrl) => {
                            let ack = (self.on_pyro_ctrl)(pyro_ctrl);
                            let encoded = crate::codec::encode_package(&mut buffer, ack);
                            serial.write(encoded).await?;
                        }
                        _ => {}
                    },
                    Err(err) => {
                        warn!("Error decoding package: {:?}", err);
                    }
                }
            };
        }
    }
}
