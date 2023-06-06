#![cfg_attr(not(test), no_std)]
#![feature(try_blocks)]
#![feature(async_fn_in_trait)]
#![feature(let_chains)]

pub use codec::{decode_package, encode_package, DecodedPackage};
pub use master::{Master, RequestError};
pub use master_drivers::{MasterGPS, MasterHarwareArming, MasterPyroContinuity, MasterPyroCtrl};
pub use packages::ack::Ack;
pub use packages::continuity::ContinuityInfo;
pub use packages::device::{DeviceInfo, GetDevice};
pub use packages::event::{Event, EventPackage, PollEvent};
pub use packages::pyro::PyroCtrl;
pub use slave::Slave;

mod codec;
mod master;
mod master_drivers;
mod multi_waker;
mod packages;
mod slave;
mod utils;

#[cfg(test)]
mod tests {
    use crate::{
        codec::{decode_package, encode_package},
        packages::device::GetDevice,
    };

    #[test]
    fn codec() {
        let get_device = GetDevice {};
        let mut buffer = [0u8; 128];
        let encoded = encode_package(&mut buffer, get_device);
        let decoded = decode_package(encoded).unwrap();
        println!("{:?}", decoded);
    }
}
