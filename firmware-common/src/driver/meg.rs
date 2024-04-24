use core::fmt::{Write, Debug};
use heapless::String;
use rkyv::{Archive, Deserialize, Serialize};

use embedded_hal_async::delay::DelayNs;

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
    type Error: defmt::Format + Debug;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<MegReading, Self::Error>;
}

pub struct DummyMegnetometer<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyMegnetometer<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Megnetometer for DummyMegnetometer<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<MegReading, ()> {
        self.delay.delay_ms(1).await;
        Ok(MegReading {
            timestamp: 0.0,
            meg: [0.0, 0.0, 0.0],
        })
    }
}
