use anyhow::Result;
use firmware_common::common::{
    device_config::{DeviceConfig, DeviceModeConfig, LoraConfig},
    rkyv_structs::RkyvString,
};
use serde::{Deserialize, Serialize};

use super::flight_profile::PyroSelectionSerde;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceConfigSerde {
    pub name: String,
    pub mode: DeviceModeConfigSerde,
    pub lora: LoraConfigSerde,
    pub lora_key: [u8; 32],
}

impl Into<DeviceConfig> for DeviceConfigSerde {
    fn into(self) -> DeviceConfig {
        let mut name = self.name.as_bytes();
        if name.len() > 64 {
            name = &name[..64];
        }
        DeviceConfig {
            name: RkyvString::from_slice(name),
            mode: self.mode.into(),
            lora: self.lora.into(),
            lora_key: self.lora_key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoraConfigSerde {
    pub frequency: u32,
    pub sf: u8,
    pub bw: u32,
    pub cr: u8,
    pub power: i32,
}

impl Into<LoraConfig> for LoraConfigSerde {
    fn into(self) -> LoraConfig {
        LoraConfig {
            frequency: self.frequency,
            sf: self.sf,
            bw: self.bw,
            cr: self.cr,
            power: self.power,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "mode")]
pub enum DeviceModeConfigSerde {
    Avionics,
    GCM,
    GroundTestAvionics {
        drogue_pyro: PyroSelectionSerde,
        main_pyro: PyroSelectionSerde,
    },
    VacuumTest,
}

impl Into<DeviceModeConfig> for DeviceModeConfigSerde {
    fn into(self) -> DeviceModeConfig {
        match self {
            DeviceModeConfigSerde::Avionics => DeviceModeConfig::Avionics,
            DeviceModeConfigSerde::GCM => DeviceModeConfig::GCM,
            DeviceModeConfigSerde::GroundTestAvionics {
                drogue_pyro,
                main_pyro,
            } => DeviceModeConfig::GroundTestAvionics {
                drogue_pyro: drogue_pyro.into(),
                main_pyro: main_pyro.into(),
            },
            DeviceModeConfigSerde::VacuumTest => DeviceModeConfig::VacuumTest,
        }
    }
}

pub fn json_to_device_config(json: String) -> Result<DeviceConfig> {
    let config: DeviceConfigSerde = serde_json::from_str(&json)?;
    Ok(config.into())
}