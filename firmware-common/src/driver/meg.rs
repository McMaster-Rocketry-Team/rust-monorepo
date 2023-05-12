use core::fmt::Write;
use heapless::String;
use nalgebra::Vector3;

use super::timer::Timer;

pub struct MegReading {
    pub timestamp: u64,    // ms
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
    type Error;
    async fn reset(&mut self, power_saving: bool) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<MegReading, Self::Error>;
}

pub struct DummyMegnetometer<T: Timer> {
    timer: T,
}

impl<T: Timer> Megnetometer for DummyMegnetometer<T> {
    type Error = ();

    async fn reset(&mut self, _power_saving: bool) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<MegReading, ()> {
        self.timer.sleep(1).await;
        Ok(MegReading {
            timestamp: 0,
            meg: Vector3::new(0.0, 0.0, 0.0),
        })
    }
}
