#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "rkyv-no-std",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct IMUReading {
    pub timestamp: f64, // ms
    pub acc: [f32; 3],  // m/s^2
    pub gyro: [f32; 3],
}
