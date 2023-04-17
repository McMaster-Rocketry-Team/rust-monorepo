use micromath::vector::F32x3;
use core::fmt::Write;
use heapless::String;
pub struct MegReading {
    pub timestamp: u64, // ms
    pub meg: F32x3,
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
    async fn read(&mut self) -> Result<MegReading,()>;
}