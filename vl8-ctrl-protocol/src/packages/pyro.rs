use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct PyroCtrl {
    pub pyro_channel: u8,
    pub enable: bool,
}

impl Package for PyroCtrl {
    fn get_id() -> u8 {
        0x01
    }
}
