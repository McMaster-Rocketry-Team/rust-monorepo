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
    pub drouge_to_main_ms: f64,
    pub main_to_landed_ms: f64,
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
            drouge_to_main_ms: self.drouge_to_main_ms,
            main_to_landed_ms: self.main_to_landed_ms,
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

pub fn json_to_flight_profile(json: String) -> Result<FlightProfile> {
    let profile: FlightProfileSerde = serde_json::from_str(&json)?;
    Ok(profile.into())
}
