use core::fmt::Write;
use heapless::String;
use nalgebra::Vector3;
use rkyv::{Archive, Deserialize, Serialize};

use super::timer::Timer;

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct MegReading {
    pub timestamp: f64, // ms
    pub meg: [f32; 3],  // gauss
}

impl defmt::Format for MegReading {
    fn format(&self, f: defmt::Formatter) {
        let mut message = String::<128>::new();
        core::write!(
            &mut message,
            "MegReading {{ {:.5} {:.5} {:.5} }}",
            self.meg[0],
            self.meg[1],
            self.meg[2],
        )
        .unwrap();
        defmt::write!(f, "{}", message.as_str())
    }
}

pub trait Megnetometer {
    type Error: defmt::Format;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<MegReading, Self::Error>;
}

pub struct DummyMegnetometer<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyMegnetometer<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> Megnetometer for DummyMegnetometer<T> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<MegReading, ()> {
        self.timer.sleep(1.0).await;
        Ok(MegReading {
            timestamp: 0.0,
            meg: [0.0, 0.0, 0.0],
        })
    }
}
