use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct IMUCalibrationInfo {
    pub gyro_offset: [f32; 3],
    pub up_right_vector: [f32; 3],
}
