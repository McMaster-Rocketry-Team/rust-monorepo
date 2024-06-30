use core::{fmt::{Debug, Write}, marker::PhantomData};
use heapless::String;
use rkyv::{Archive, Deserialize, Serialize};

use embedded_hal_async::delay::DelayNs;

use crate::{common::{delta_factory::Deltable, unix_clock::UnixClock}, Clock};

use super::timestamp::{BootTimestamp, TimestampType, UnixTimestamp};

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct MagReading<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: f64, // ms
    pub mag: [f32; 3],  // gauss
}

impl MagReading<BootTimestamp> {
    pub fn to_unix_timestamp(
        self,
        unix_clock: UnixClock<impl Clock>,
    ) -> MagReading<UnixTimestamp> {
        MagReading {
            _phantom: PhantomData,
            timestamp: unix_clock.convert_to_unix(self.timestamp),
            mag: self.mag,
        }
    }
}

impl<T: TimestampType> MagReading<T> {
    pub fn new(timestamp: f64, mag: [f32; 3]) -> Self {
        Self {
            _phantom: PhantomData,
            timestamp,
            mag,
        }
    }
}

impl<T: TimestampType> defmt::Format for MagReading<T> {
    fn format(&self, f: defmt::Formatter) {
        let mut message = String::<128>::new();
        core::write!(
            &mut message,
            "MagReading {{ {:.5} {:.5} {:.5} }}",
            self.mag[0],
            self.mag[1],
            self.mag[2],
        )
        .unwrap();
        defmt::write!(f, "{}", message.as_str())
    }
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct MagReadingDelta<T: TimestampType> {
    _phantom: PhantomData<T>,
    pub timestamp: u8,
    pub mag: [u8; 3],
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 10.0, f64, u8);
    fixed_point_factory!(Mag, -0.1, 0.1, f32, u8);
}

impl<T: TimestampType> Deltable for MagReading<T> {
    type DeltaType = MagReadingDelta<T>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            _phantom: PhantomData,
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            mag: [
                self.mag[0] + factories::Mag::to_float(delta.mag[0]),
                self.mag[1] + factories::Mag::to_float(delta.mag[1]),
                self.mag[2] + factories::Mag::to_float(delta.mag[2]),
            ],
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(MagReadingDelta {
            _phantom: PhantomData,
            timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
            mag: [
                factories::Mag::to_fixed_point(self.mag[0] - other.mag[0])?,
                factories::Mag::to_fixed_point(self.mag[1] - other.mag[1])?,
                factories::Mag::to_fixed_point(self.mag[2] - other.mag[2])?,
            ],
        })
    }
}


pub trait Magnetometer {
    type Error: defmt::Format + Debug;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<MagReading<BootTimestamp>, Self::Error>;
}

pub struct DummyMagnetometer<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyMagnetometer<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> Magnetometer for DummyMagnetometer<D> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<MagReading<BootTimestamp>, ()> {
        self.delay.delay_ms(1).await;
        Ok(MagReading {
            _phantom: PhantomData,
            timestamp: 0.0,
            mag: [0.0, 0.0, 0.0],
        })
    }
}
