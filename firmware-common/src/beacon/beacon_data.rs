use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, defmt::Format)]
pub struct BeaconData {
    pub satellite_count: Option<u8>,
    pub longitude: Option<f32>,
    pub latitude: Option<f32>,
}
