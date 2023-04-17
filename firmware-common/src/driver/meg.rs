use core::fmt::Write;
use heapless::String;
use nalgebra::Vector3;

pub struct MegReading {
    pub timestamp: u64, // ms
    pub meg: Vector3<f32>, // gauss
}

impl defmt::Format for MegReading {
    fn format(&self, f: defmt::Formatter) {
        let mut message = String::<128>::new();
        core::write!(
            &mut message,
            "MegReading {{ {:.2} {:.2} {:.2} }}",
            self.meg.x,
            self.meg.y,
            self.meg.z,
        )
        .unwrap();
        defmt::write!(f, "{}", message.as_str())
    }
}

pub trait Megnetometer {
    async fn reset(&mut self, power_saving:bool) -> Result<(), ()>;

    async fn read(&mut self) -> Result<MegReading, ()>;
}
