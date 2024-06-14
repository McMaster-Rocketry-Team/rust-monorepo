use rkyv::{Archive, Deserialize, Serialize};

use super::rkyv_structs::{RkyvString, RkyvVec};

#[derive(Clone, Debug, defmt::Format, Archive, Serialize, Deserialize)]
pub struct LoraConfig {
    pub frequencies: RkyvVec<64, u32>,
    pub sf: u8,
    pub bw: u32,
    pub cr: u8,
}

impl Into<lora_modulation::BaseBandModulationParams> for LoraConfig {
    fn into(self) -> lora_modulation::BaseBandModulationParams {
        use lora_modulation::{Bandwidth, BaseBandModulationParams, CodingRate, SpreadingFactor};

        BaseBandModulationParams::new(
            match self.sf {
                5 => SpreadingFactor::_5,
                6 => SpreadingFactor::_6,
                7 => SpreadingFactor::_7,
                8 => SpreadingFactor::_8,
                9 => SpreadingFactor::_9,
                10 => SpreadingFactor::_10,
                11 => SpreadingFactor::_11,
                12 => SpreadingFactor::_12,
                _ => panic!("Invalid spreading factor"),
            },
            match self.bw {
                7810u32 => Bandwidth::_7KHz,
                10420u32 => Bandwidth::_10KHz,
                15630u32 => Bandwidth::_15KHz,
                20830u32 => Bandwidth::_20KHz,
                31250u32 => Bandwidth::_31KHz,
                41670u32 => Bandwidth::_41KHz,
                62500u32 => Bandwidth::_62KHz,
                125000u32 => Bandwidth::_125KHz,
                250000u32 => Bandwidth::_250KHz,
                500000u32 => Bandwidth::_500KHz,
                _ => panic!("Invalid bandwidth"),
            },
            match self.cr {
                5 => CodingRate::_4_5,
                6 => CodingRate::_4_6,
                7 => CodingRate::_4_7,
                8 => CodingRate::_4_8,
                _ => panic!("Invalid coding rate"),
            },
        )
    }
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
