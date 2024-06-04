use core::{fmt::{Debug, Write}, marker::PhantomData};
use heapless::String;
use rkyv::{Archive, Deserialize, Serialize};

use embedded_hal_async::delay::DelayNs;

use crate::common::delta_factory::Deltable;

use super::timestamp::{BootTimestamp, TimestampType};

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct MegReading<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: f64, // ms
    pub meg: [f32; 3],  // gauss
}

impl<T: TimestampType> defmt::Format for MegReading<T> {
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

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct MegReadingDelta<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: u8,
    pub meg: [u8; 3],
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 10.0, f64, u8);
    fixed_point_factory!(Meg, -0.1, 0.1, f32, u8);
}

impl<T: TimestampType> Deltable for MegReading<T> {
    type DeltaType = MegReadingDelta<T>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            _phantom: PhantomData,
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            meg: [
                self.meg[0] + factories::Meg::to_float(delta.meg[0]),
                self.meg[1] + factories::Meg::to_float(delta.meg[1]),
                self.meg[2] + factories::Meg::to_float(delta.meg[2]),
            ],
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(MegReadingDelta {
            _phantom: PhantomData,
            timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
            meg: [
                factories::Meg::to_fixed_point(self.meg[0] - other.meg[0])?,
                factories::Meg::to_fixed_point(self.meg[1] - other.meg[1])?,
                factories::Meg::to_fixed_point(self.meg[2] - other.meg[2])?,
            ],
        })
    }
}


pub trait Megnetometer {
    type Error: defmt::Format + Debug;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<MegReading<BootTimestamp>, Self::Error>;
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

    async fn read(&mut self) -> Result<MegReading<BootTimestamp>, ()> {
        self.delay.delay_ms(1).await;
        Ok(MegReading {
            _phantom: PhantomData,
            timestamp: 0.0,
            meg: [0.0, 0.0, 0.0],
        })
    }
}
