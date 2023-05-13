use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct IMUReading {
    pub timestamp: u64,    // ms
    pub acc: Vector3<f32>, // m/s^2
    pub gyro: Vector3<f32>,
}
