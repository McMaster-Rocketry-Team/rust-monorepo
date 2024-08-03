use std::{fs::read_to_string, path::Path};

use anyhow::Result;
use firmware_common::common::{
    device_config::{DeviceConfig, DeviceModeConfig, LoraConfig},
    rkyv_structs::RkyvString,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::flight_profile::PyroSelectionSerde;

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

pub fn read_device_config<P: AsRef<Path>>(path: P) -> Result<DeviceConfig> {
    let config = read_to_string(path)?;
    let config: DeviceConfigSerde = serde_json::from_str(&config)?;
    Ok(config.into())
}

pub fn gen_lora_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}

pub fn format_lora_key(key: &[u8; 32]) -> String {
    let mut formatted = String::new();
    formatted.push('[');
    for (i, byte) in key.iter().enumerate() {
        formatted.push_str(&format!("{}", byte));
        if i < key.len() - 1 {
            formatted.push(',');
            formatted.push(' ');
        }
    }
    formatted.push(']');
    formatted
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_config() {
        let config = read_device_config("./test-configs/vacuum-test.json").unwrap();
        println!("{:?}", config);
    }

    #[test]
    fn print_lora_key() {
        let key = gen_lora_key();
        println!("{}", format_lora_key(&key));
    }
}
