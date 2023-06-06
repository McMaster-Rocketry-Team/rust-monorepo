use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct GetHardwareArming {
}

impl Package for GetHardwareArming {
    fn get_id() -> u8 {
        0x08
    }
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct HardwareArmingInfo {
    pub armed: bool,
}

impl Package for HardwareArmingInfo {
    fn get_id() -> u8 {
        0x09
    }
}
