use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use firmware_common::driver::serial::Serial;

use crate::{
    codec::{decode_package, encode_package, DecodePackageError, DecodedPackage},
    packages::Package,
};

pub struct Master<S: Serial> {
    serial: Mutex<CriticalSectionRawMutex, S>,
}

pub enum RequestError<S: Serial> {
    SerialError(S::Error),
    PackageError(DecodePackageError),
}

impl<S: Serial> Master<S> {
    pub fn new(serial: S) -> Self {
        Self {
            serial: Mutex::new(serial),
        }
    }

    pub async fn request(&self, package: impl Package) -> Result<DecodedPackage, RequestError<S>> {
        let mut serial = self.serial.lock().await;
        let mut buffer = [0u8; 128];
        let encoded = encode_package(&mut buffer, package);
        serial
            .write(encoded)
            .await
            .map_err(RequestError::SerialError)?;

        // TODO retry on timeout
        let len = serial
            .read(&mut buffer)
            .await
            .map_err(RequestError::SerialError)?;
        decode_package(&buffer[..len]).map_err(RequestError::PackageError)
    }
}
