use bilge::prelude::*;

#[bitsize(66)]
pub struct AvionicsStatus {
    timestamp: [u8; 8],
    low_power: bool,
    armed: bool,
}


pub struct Ignition;

pub struct Apogee;

pub struct Landed;
