use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct GetDevice {}

impl Package for GetDevice {
    fn get_id() -> u8 {
        0x02
    }
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct DeviceInfo {
    pub device_id: [u8; 12],
    pub device_model: u8,
}

impl Package for DeviceInfo {
    fn get_id() -> u8 {
        0x03
    }
}
