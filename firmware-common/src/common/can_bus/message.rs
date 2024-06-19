use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct AvionicsStatus {
    pub timestamp: f64,
    pub low_power: bool,
    pub armed: bool,
}

impl AvionicsStatus {
    pub fn message_id() -> u32 {
        0xA0
    }
}

pub const IGNITION_MESSAGE_ID: u32 = 0xF0;

pub const APOGEE_MESSAGE_ID: u32 = 0xF1;

pub const LANDED_MESSAGE_ID: u32 = 0xF2;

pub const SOLAR_CAR_DAQ_HEALTH_MESSAGE_ID: u32 = 0xB0;

pub const STRAIN_GAUGES_HEALTH_MESSAGE_ID: u32 = 0xB1;
