use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct Ack {}

impl Package for Ack {
    fn get_id() -> u8 {
        0x00
    }
}
