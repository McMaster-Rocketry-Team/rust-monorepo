use std::{fs::read_to_string, path::Path};

use anyhow::Result;
use firmware_common::avionics::flight_profile::{FlightProfile, PyroSelection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlightProfileSerde {
    pub drogue_pyro: PyroSelectionSerde,
    pub drogue_chute_minimum_time_ms: f64,
    pub drogue_chute_minimum_altitude_agl: f32,
    pub drogue_chute_delay_ms: f64,
    pub main_pyro: PyroSelectionSerde,
    pub main_chute_altitude_agl: f32,
    pub main_chute_delay_ms: f64,
}

impl Into<FlightProfile> for FlightProfileSerde {
    fn into(self) -> FlightProfile {
        FlightProfile {
            drogue_pyro: self.drogue_pyro.into(),
            drogue_chute_minimum_time_ms: self.drogue_chute_minimum_time_ms,
            drogue_chute_minimum_altitude_agl: self.drogue_chute_minimum_altitude_agl,
            drogue_chute_delay_ms: self.drogue_chute_delay_ms,
            main_pyro: self.main_pyro.into(),
            main_chute_altitude_agl: self.main_chute_altitude_agl,
            main_chute_delay_ms: self.main_chute_delay_ms,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PyroSelectionSerde {
    Pyro1 = 1,
    Pyro2 = 2,
    Pyro3 = 3,
}

impl Into<PyroSelection> for PyroSelectionSerde {
    fn into(self) -> PyroSelection {
        match self {
            PyroSelectionSerde::Pyro1 => PyroSelection::Pyro1,
            PyroSelectionSerde::Pyro2 => PyroSelection::Pyro2,
            PyroSelectionSerde::Pyro3 => PyroSelection::Pyro3,
        }
    }
}

pub fn read_flight_profile<P: AsRef<Path>>(path: P) -> Result<FlightProfile> {
    let profile = read_to_string(path)?;
    let profile: FlightProfileSerde = serde_json::from_str(&profile)?;
    Ok(profile.into())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_flight_profile() {
        let profile = read_flight_profile("./test-configs/flight-profile.json").unwrap();
        println!("{:?}", profile);
    }
}
