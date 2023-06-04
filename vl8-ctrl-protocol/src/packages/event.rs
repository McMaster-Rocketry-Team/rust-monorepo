use rkyv::{Archive, Deserialize, Serialize};

use super::Package;

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
    Continuity { pyro_id: u8, continuity: bool },
    HardwareArming { armed: bool },
    NMEAMessage { message: [u8; 82] },
}

#[derive(Archive, Deserialize, Serialize, defmt::Format, Debug)]
#[archive(check_bytes)]
pub struct EventPackage {
    events_left: u8,
    event: Event,
}

impl Package for EventPackage {
    fn get_id() -> u8 {
        0x05
    }
}
