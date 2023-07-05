use super::Package;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct PollEvent {}

impl Package for PollEvent {
    fn get_id() -> u8 {
        0x04
    }
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub enum Event {
    Continuity { pyro_channel: u8, continuity: bool },
    HardwareArming { armed: bool },
    NmeaSentence { sentence: [u8; 84], length: u8 },
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct EventPackage {
    pub events_left: u8,
    pub event: Option<Event>,
}

impl Package for EventPackage {
    fn get_id() -> u8 {
        0x05
    }
}
