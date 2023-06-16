use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Copy, Clone, defmt::Format)]
pub enum AvionicsState {
    Sleeping,
    SoftArmed,
    PowerAscent,
    Coasting,
    Descent,
    Landed,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
pub struct TelemetryData {
    timestamp: u64, // ms
    avionics_state: AvionicsState,
    position: [f32; 3],
    pressure: f32,
    temperature: f32,
    satellites_in_use: i32,
    lat_lon: Option<(f32, f32)>,
    armed: bool,
    battery_voltage: f32,
}
