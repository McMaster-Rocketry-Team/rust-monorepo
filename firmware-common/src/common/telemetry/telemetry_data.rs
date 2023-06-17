use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Copy, Clone, defmt::Format, Default)]
pub enum AvionicsState {
    #[default]
    Idle,
    Armed,
    PowerAscent,
    Coasting,
    Descent,
    Landed,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Default, defmt::Format)]
pub struct TelemetryData {
    pub timestamp: f64, // ms
    pub avionics_state: AvionicsState,
    pub altitude: f32,
    pub pressure: f32,
    pub temperature: f32,
    pub satellites_in_use: u32,
    pub lat_lon: Option<(f64, f64)>,
    pub battery_voltage: f32,
    pub pyro1_cont: bool,
    pub pyro2_cont: bool,
}
