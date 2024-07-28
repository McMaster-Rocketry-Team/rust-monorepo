use crate::{
    driver::timestamp::{BootTimestamp, TimestampType, UnixTimestamp},
    Clock,
};

use super::{
    delta_factory::Deltable,
    delta_logger::bitslice_io::{BitArrayDeserializable, BitArraySerializable},
    unix_clock::UnixClock,
};

pub trait SensorReading<T: TimestampType>: Sized + Clone {
    type Data: BitArraySerializable
        + BitArrayDeserializable
        + Deltable<DeltaType: BitArraySerializable + BitArrayDeserializable>;
    type NewType<NT: TimestampType>: SensorReading<NT>;

    fn new<NT: TimestampType>(timestamp: f64, data: Self::Data) -> Self::NewType<NT>;

    fn get_timestamp(&self) -> f64;
    fn get_data(&self) -> &Self::Data;
    fn into_data(self) -> Self::Data;
}

pub fn sensor_reading_to_unix_timestamp<T: SensorReading<BootTimestamp>>(
    unix_clock: &UnixClock<impl Clock>,
    reading: T,
) -> T::NewType<UnixTimestamp> {
    T::new(
        unix_clock.convert_to_unix(reading.get_timestamp()),
        reading.into_data(),
    )
}
