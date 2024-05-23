use core::cell::RefCell;
use core::task::Waker;

use defmt::{warn, Format};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_hal_async::delay::DelayNs;
use firmware_common::driver::serial::Serial;

use crate::multi_waker::MultiWakerRegistration;
use crate::packages::camera::CameraCtrl;
use crate::packages::continuity::{ContinuityInfo, GetContinuity};
use crate::packages::hardware_arming::{GetHardwareArming, HardwareArmingInfo};
use crate::{
    codec::{decode_package, encode_package, DecodePackageError, DecodedPackage},
    packages::Package,
    utils::run_with_timeout,
    DeviceInfo, Event, GetDevice, PollEvent, PyroCtrl,
};

pub struct Master<S: Serial, D: DelayNs + Copy> {
    serial: Mutex<NoopRawMutex, S>,
    delay: D,
    pub(crate) last_event: BlockingMutex<NoopRawMutex, RefCell<Option<Event>>>,
    pub(crate) wakers_reg: BlockingMutex<NoopRawMutex, RefCell<MultiWakerRegistration<10>>>,
}

#[derive(Debug, Format)]
pub enum RequestError<E: Format> {
    SerialError(E),
    PackageError(DecodePackageError),
    ProtocolError,
    Timeout,
}

impl<S: Serial, D: DelayNs + Copy> Master<S, D> {
    pub fn new(serial: S, delay: D) -> Self {
        Self {
            serial: Mutex::new(serial),
            delay,
            last_event: BlockingMutex::new(RefCell::new(None)),
            wakers_reg: BlockingMutex::new(RefCell::new(MultiWakerRegistration::new())),
        }
    }

    async fn request_once(
        &self,
        serial: &mut impl Serial<Error = S::Error>,
        encoded_package: &[u8],
        buffer: &mut [u8],
    ) -> Result<DecodedPackage, RequestError<S::Error>> {
        serial
            .write(encoded_package)
            .await
            .map_err(RequestError::SerialError)?;

        let len = match run_with_timeout(self.delay, 30.0, serial.read(buffer)).await {
            Ok(Ok(len)) => len,
            Ok(Err(e)) => return Err(RequestError::SerialError(e)),
            Err(_) => return Err(RequestError::Timeout),
        };

        decode_package(&buffer[..len]).map_err(RequestError::PackageError)
    }

    pub async fn request(
        &self,
        package: impl Package,
    ) -> Result<DecodedPackage, RequestError<S::Error>> {
        let mut buffer = [0u8; 128];
        let mut serial = self.serial.lock().await;
        let encoded = encode_package(&mut buffer, package).clone();
        let mut buffer = [0u8; 128];

        let mut i = 0u32;
        loop {
            let result = self.request_once(&mut serial, encoded, &mut buffer).await;
            if result.is_ok() || i > 3 {
                return result;
            }
            i += 1;
        }
    }

    pub async fn poll_event(&self) -> Result<u8, RequestError<S::Error>> {
        let response = self.request(PollEvent {}).await?;
        match response {
            DecodedPackage::EventPackage(event_package) => {
                if event_package.event.is_some() {
                    self.last_event.lock(|last_event| {
                        *last_event.borrow_mut() = event_package.event;
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

    pub async fn get_device(&self) -> Result<DeviceInfo, RequestError<S::Error>> {
        let response = self.request(GetDevice {}).await?;
        match response {
            DecodedPackage::DeviceInfo(device_info) => Ok(device_info),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub(crate) async fn pyro_ctrl(
        &self,
        pyro_channel: u8,
        enable: bool,
    ) -> Result<(), RequestError<S::Error>> {
        let response = self
            .request(PyroCtrl {
                pyro_channel,
                enable,
            })
            .await?;
        match response {
            DecodedPackage::Ack(_) => Ok(()),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub(crate) async fn camera_ctrl(
        &self,
        is_recording: bool,
    ) -> Result<(), RequestError<S::Error>> {
        let response = self.request(CameraCtrl { is_recording }).await?;
        match response {
            DecodedPackage::Ack(_) => Ok(()),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub(crate) async fn get_continuity(&self, channel: u8) -> Result<bool, RequestError<S::Error>> {
        let response = self
            .request(GetContinuity {
                pyro_channel: channel,
            })
            .await?;
        match response {
            DecodedPackage::ContinuityInfo(ContinuityInfo { continuity }) => Ok(continuity),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub(crate) async fn get_hardware_arming(&self) -> Result<bool, RequestError<S::Error>> {
        let response = self.request(GetHardwareArming {}).await?;
        match response {
            DecodedPackage::HardwareArmingInfo(HardwareArmingInfo { armed }) => Ok(armed),
            _ => Err(RequestError::ProtocolError),
        }
    }

    pub(crate) fn register_waker(&self, waker: &Waker) {
        self.wakers_reg.lock(|reg| {
            if let Err(_) = reg.borrow_mut().register(waker) {
                warn!("Failed to register waker");
            }
        });
    }
}
