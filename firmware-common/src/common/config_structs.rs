use rkyv::{Archive, Deserialize, Serialize};

use super::rkyv_structs::{RkyvString, RkyvVec};

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct LoraConfig {
    pub frequencies: RkyvVec<64, u32>,
    pub sf: u8,
    pub bw: u32,
    pub cr: u8,
}

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub enum DeviceModeConfig {
    Avionics,
    GCM,
    BeaconSender,
    BeaconReceiver,
    GroundTestAvionics,
    GroundTestGCM,
}

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: RkyvString<64>,
    pub mode: DeviceModeConfig,
    pub lora: LoraConfig,
}
