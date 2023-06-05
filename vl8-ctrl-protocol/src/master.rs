use core::cell::RefCell;

use defmt::{warn, Format};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::waitqueue::MultiWakerRegistration;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use firmware_common::driver::{serial::Serial, timer::Timer};

use crate::{
    codec::{decode_package, encode_package, DecodePackageError, DecodedPackage},
    packages::Package,
    utils::run_with_timeout,
    DeviceInfo, Event, GetDevice, PollEvent, PyroCtrl,
};

pub struct Master<S: Serial, T: Timer> {
    serial: Mutex<CriticalSectionRawMutex, S>,
    timer: T,
    pub(crate) last_event: BlockingMutex<CriticalSectionRawMutex, RefCell<Option<Event>>>,
    pub(crate) wakers_reg:
        BlockingMutex<CriticalSectionRawMutex, RefCell<MultiWakerRegistration<10>>>,
}

#[derive(Debug, Format)]
pub enum RequestError<E: Format> {
    SerialError(E),
    PackageError(DecodePackageError),
    ProtocolError,
    Timeout,
}

impl<S: Serial, T: Timer> Master<S, T> {
    pub fn new(serial: S, timer: T) -> Self {
        Self {
            serial: Mutex::new(serial),
            timer,
            last_event: BlockingMutex::new(RefCell::new(None)),
            wakers_reg: BlockingMutex::new(RefCell::new(MultiWakerRegistration::new())),
        }
    }

    pub async fn request(
        &self,
        package: impl Package,
    ) -> Result<DecodedPackage, RequestError<S::Error>> {
        let mut buffer = [0u8; 128];
        let mut serial = self.serial.lock().await;
        let encoded = encode_package(&mut buffer, package);
        serial
            .write(encoded)
            .await
            .map_err(RequestError::SerialError)?;

        let len = match run_with_timeout(self.timer, 2.0, serial.read(&mut buffer)).await {
            Ok(Ok(len)) => len,
            Ok(Err(e)) => return Err(RequestError::SerialError(e)),
            Err(_) => return Err(RequestError::Timeout),
        };
        decode_package(&buffer[..len]).map_err(RequestError::PackageError)
    }

    pub async fn poll_event(&self) -> Result<u8, RequestError<S::Error>> {
        let response = self.request(PollEvent {}).await?;
        match response {
            DecodedPackage::EventPackage(event_package) => {
                if event_package.event.is_some() {
                    self.last_event.lock(|last_event| {
                        let mut last_event = last_event.borrow_mut();
                        if last_event.is_some() {
                            warn!(
                                "Received a new event but the previous one was not consumed: {}",
                                last_event.take().unwrap()
                            );
                        }
                        *last_event = event_package.event;
                    });
                    self.wakers_reg.lock(|reg| {
                        reg.borrow_mut().wake();
                    });
                }

                Ok(event_package.events_left)
            }
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub async fn pyro_ctrl(&self, pyro_channel: u8, enable: bool) -> Result<(), RequestError<S::Error>> {
        let response = self.request(PyroCtrl { pyro_channel, enable }).await?;
        match response {
            DecodedPackage::Ack(_) => Ok(()),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub async fn get_device(&self) -> Result<DeviceInfo, RequestError<S::Error>> {
        let response = self.request(GetDevice {}).await?;
        match response {
            DecodedPackage::DeviceInfo(device_info) => Ok(device_info),
            _ => Err(RequestError::ProtocolError),
        }
    }
}
