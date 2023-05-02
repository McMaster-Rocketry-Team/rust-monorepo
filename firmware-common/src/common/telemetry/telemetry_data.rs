use nalgebra::{Vector3, Vector4};

use crate::driver::pyro;

#[derive(Debug, Copy, Clone)]
pub enum AvionicsState {
    Sleeping,
    SoftArmed,
    PowerAscent,
    Coasting,
    Descent,
    Landed,
}

#[derive(Debug, Clone)]
pub struct Pyro {
    continuity: bool,
    firing: bool,
    fault: bool,
}

#[derive(Debug, Clone)]
pub struct TelemetryData {
    timestamp: u64, // ms
    avionics_state: AvionicsState,
    acceleration: Option<Vector3<f32>>,
    rotation_speed: Option<Vector3<f32>>,
    orientation: Option<Vector4<f32>>,
    speed: Option<Vector3<f32>>,
    position: Option<Vector3<f32>>,
    pressure: Option<f32>,
    air_temperature: Option<f32>,
    cpu_temperature: f32,
    satellites_in_use: i32,
    lat_lon: Option<(f32, f32)>,
    armed: bool,
    soft_armed: bool,
    pyro1: Pyro,
    pyro2: Pyro,
    pyro3: Pyro,
    battery_voltage: f32,
    current: f32,
    buzzer: bool,
}
