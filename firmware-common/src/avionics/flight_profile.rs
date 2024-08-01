use int_enum::IntEnum;
use rkyv::{Archive, Deserialize, Serialize};

#[repr(u8)]
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Archive, Serialize, Deserialize, IntEnum)]
pub enum PyroSelection {
    Pyro1 = 1,
    Pyro2 = 2,
    Pyro3 = 3,
}

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct FlightProfile {
    pub drogue_pyro: PyroSelection,
    pub drogue_chute_minimum_time_ms: f64,
    pub drogue_chute_minimum_altitude_agl: f32,
    pub drogue_chute_delay_ms: f64,
    pub main_pyro: PyroSelection,
    pub main_chute_altitude_agl: f32,
    pub main_chute_delay_ms: f64,
}
