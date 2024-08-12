use crate::{
    common::sensor_reading::{SensorData, SensorReading},
    driver::timestamp::BootTimestamp,
};

use super::delta_logger::UnixTimestampLog;

pub trait DeltaLoggerTrait<D: SensorData, I> {
    type Error: core::fmt::Debug + defmt::Format;

    /// returns true if the reading was logged
    async fn log(&mut self, reading: SensorReading<BootTimestamp, D>) -> Result<bool, Self::Error>;

    async fn log_unix_time(&mut self, log: UnixTimestampLog) -> Result<(), Self::Error>;

    async fn flush(&mut self) -> Result<(), Self::Error>;

    /// Must call flush before calling this
    async fn into_inner(self) -> Result<I, Self::Error>;
}