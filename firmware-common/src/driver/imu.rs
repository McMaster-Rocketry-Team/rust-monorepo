use micromath::vector::F32x3;

pub struct IMUReading {
    pub timestamp: u64, // ms
    pub acc: F32x3,
    pub gyro: F32x3,
}

impl defmt::Format for IMUReading {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "IMUReading {{ acc: {} {} {}, gyro: {} {} {} }}",
            self.acc.x,
            self.acc.y,
            self.acc.z,
            self.gyro.x,
            self.gyro.y,
            self.gyro.z,
        )
    }
}

pub trait IMU {
    async fn read(&mut self) -> IMUReading;
}
