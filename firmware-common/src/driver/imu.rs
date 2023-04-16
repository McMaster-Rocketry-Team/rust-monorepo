use core::fmt::Write;
use heapless::String;
use micromath::vector::F32x3;
#[allow(unused_imports)]
use micromath::F32Ext;

pub struct IMUReading {
    pub timestamp: u64, // ms
    pub acc: F32x3,
    pub gyro: F32x3,
}

impl defmt::Format for IMUReading {
    fn format(&self, f: defmt::Formatter) {
        let mut message = String::<128>::new();
        core::write!(
            &mut message,
            "IMUReading {{ acc: {:.2} {:.2} {:.2}, gyro: {:.2} {:.2} {:.2} }}",
            self.acc.x,
            self.acc.y,
            self.acc.z,
            self.gyro.x,
            self.gyro.y,
            self.gyro.z,
        )
        .unwrap();
        defmt::write!(f, "{}", message.as_str())
    }
}

pub trait IMU {
    async fn read(&mut self) -> IMUReading;
}
