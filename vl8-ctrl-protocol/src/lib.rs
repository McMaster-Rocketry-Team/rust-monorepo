#![cfg_attr(not(test), no_std)]
#![feature(try_blocks)]

pub use master::{Master, RequestError};
pub use slave::Slave;
pub use packages::ack::Ack;
pub use packages::device::{DeviceInfo, GetDevice};
pub use packages::event::{Event, PollEvent, EventPackage};
pub use packages::pyro::{PyroCtrl};

mod codec;
mod master;
mod slave;
mod packages;

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
