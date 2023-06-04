use core::cell::RefCell;
use core::future::Future;

use defmt::{unwrap, warn};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use firmware_common::driver::serial::Serial;
use heapless::Deque;

use crate::codec::{decode_package, encode_package, DecodedPackage};
use crate::packages::ack::Ack;
use crate::packages::device::{DeviceInfo, GetDevice};
use crate::packages::event::Event;
use crate::packages::pyro::PyroCtrl;
use crate::packages::Package;
use crate::EventPackage;

pub struct Slave<S: Serial> {
    events: BlockingMutex<CriticalSectionRawMutex, RefCell<Deque<Event, 5>>>,
    serial: Mutex<CriticalSectionRawMutex, S>,
}

impl<S: Serial> Slave<S> {
    pub fn new(serial: S) -> Self {
        Self {
            events: BlockingMutex::new(RefCell::new(Deque::new())),
            serial: Mutex::new(serial),
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

    pub async fn respond(&self, package: impl Package) -> Result<(), S::Error> {
        let mut serial = self.serial.lock().await;
        let mut buffer = [0u8; 128];
        let encoded = encode_package(&mut buffer, package);
        serial.write(encoded).await
    }

    pub async fn next(&self) -> DecodedPackage {
        let mut buffer = [0u8; 128];
        let mut serial = self.serial.lock().await;
        loop {
            let _: Result<(), S::Error> = try {
                let len = serial.read(&mut buffer).await?;
                let package = decode_package(&buffer[..len]);
                match package {
                    Ok(package) => match package {
                        DecodedPackage::PollEvent(_) => {
                            let event_package = self.events.lock(|events| {
                                let mut events = events.borrow_mut();

                                if let Some(event) = events.pop_front() {
                                    EventPackage {
                                        events_left: events.len() as u8,
                                        event: Some(event),
                                    }
                                } else {
                                    EventPackage {
                                        events_left: 0,
                                        event: None,
                                    }
                                }
                            });

                            let encoded = encode_package(&mut buffer, event_package);
                            unwrap!(serial.write(encoded).await);
                        }
                        package => {
                            return package;
                        }
                    },
                    Err(err) => {
                        warn!("Error decoding package: {:?}", err);
                    }
                }
            };
        }
    }

    pub async fn run<A, AF, B, BF>(&self, mut on_get_device: A, mut on_pyro_ctrl: B) -> !
    where
        AF: Future<Output = DeviceInfo>,
        A: FnMut(GetDevice) -> AF,
        BF: Future<Output = Ack>,
        B: FnMut(PyroCtrl) -> BF,
    {
        let mut buffer = [0u8; 128];
        loop {
            let _: Result<(), S::Error> = try {
                let mut serial = self.serial.lock().await;
                let len = serial.read(&mut buffer).await?;
                let package = decode_package(&buffer[..len]);
                match package {
                    Ok(package) => match package {
                        DecodedPackage::GetDevice(get_device) => {
                            let device_info = on_get_device(get_device).await;
                            let encoded = crate::codec::encode_package(&mut buffer, device_info);
                            serial.write(encoded).await?;
                        }
                        DecodedPackage::PyroCtrl(pyro_ctrl) => {
                            let ack = on_pyro_ctrl(pyro_ctrl).await;
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
