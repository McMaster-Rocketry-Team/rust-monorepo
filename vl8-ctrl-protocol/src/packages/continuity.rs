use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct GetContinuity {
    pub pyro_channel: u8,
}

impl Package for GetContinuity {
    fn get_id() -> u8 {
        0x06
    }
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct ContinuityInfo {
    pub continuity: bool,
}

impl Package for ContinuityInfo {
    fn get_id() -> u8 {
        0x07
    }
}
