use core::marker::PhantomData;

use crate::driver::{clock::Clock, timestamp::{TimestampType, UnixTimestamp}};

use super::{
    delta_logger::{bitslice_serialize::BitArraySerializable, delta_factory::Deltable},
    unix_clock::UnixClock,
};

pub trait SensorData:
    BitArraySerializable
    + Deltable<DeltaType: BitArraySerializable>
    + defmt::Format
    + core::fmt::Debug
    + Clone
{
}

#[derive(defmt::Format, Debug, Clone)]
pub struct SensorReading<T: TimestampType, D: SensorData> {
    _phantom_timestamp: PhantomData<T>,
    pub timestamp: f64,
    pub data: D,
}

impl<T: TimestampType, D: SensorData> SensorReading<T, D> {
    pub fn new(timestamp: f64, data: D) -> Self {
        SensorReading {
            _phantom_timestamp: PhantomData,
            timestamp,
            data,
        }
    }

    pub fn to_unix_timestamp(
        &self,
        unix_clock: &UnixClock<impl Clock>,
    ) -> SensorReading<UnixTimestamp, D> {
        SensorReading {
            _phantom_timestamp: PhantomData,
            timestamp: unix_clock.convert_to_unix(self.timestamp),
            data: self.data.clone(),
        }
    }
}
