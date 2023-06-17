use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct CameraCtrl {
    pub is_recording: bool,
}

impl Package for CameraCtrl {
    fn get_id() -> u8 {
        0x0A
    }
}
