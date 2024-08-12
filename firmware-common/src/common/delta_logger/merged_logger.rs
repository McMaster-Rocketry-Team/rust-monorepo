use core::{fmt, marker::PhantomData};

use crate::{
    common::sensor_reading::{SensorData, SensorReading},
    driver::timestamp::BootTimestamp,
};

use super::prelude::DeltaLoggerTrait;

pub struct MergedLogger<D, I1, I2, L1, L2>
where
    D: SensorData,
    L1: DeltaLoggerTrait<D, I1>,
    L2: DeltaLoggerTrait<D, I2>,
{
    phantom_d: PhantomData<D>,
    phantom_i1: PhantomData<I1>,
    phantom_i2: PhantomData<I2>,
    logger_1: L1,
    logger_2: L2,
}

impl<D, I1, I2, L1, L2> MergedLogger<D, I1, I2, L1, L2>
where
    D: SensorData,
    L1: DeltaLoggerTrait<D, I1>,
    L2: DeltaLoggerTrait<D, I2>,
{
    pub fn new(logger_1: L1, logger_2: L2) -> Self {
        Self {
            phantom_d: PhantomData,
            phantom_i1: PhantomData,
            phantom_i2: PhantomData,
            logger_1,
            logger_2,
        }
    }
}

#[derive(Debug, defmt::Format)]
pub enum MergedLoggerError<E1: fmt::Debug + defmt::Format, E2: fmt::Debug + defmt::Format> {
    Logger1(E1),
    Logger2(E2),
    Both(E1, E2),
}

impl<E1: fmt::Debug + defmt::Format, E2: fmt::Debug + defmt::Format> MergedLoggerError<E1, E2> {
    fn from_results<T1, T2>(
        result_1: Result<T1, E1>,
        result_2: Result<T2, E2>,
    ) -> Result<(T1, T2), Self> {
        match (result_1, result_2) {
            (Ok(t1), Ok(t2)) => Ok((t1, t2)),
            (Err(e1), Ok(_)) => Err(Self::Logger1(e1)),
            (Ok(_), Err(e2)) => Err(Self::Logger2(e2)),
            (Err(e1), Err(e2)) => Err(Self::Both(e1, e2)),
        }
    }
}

impl<D, I1, I2, L1, L2> DeltaLoggerTrait<D, (L1, L2)> for MergedLogger<D, I1, I2, L1, L2>
where
    D: SensorData,
    L1: DeltaLoggerTrait<D, I1>,
    L2: DeltaLoggerTrait<D, I2>,
{
    type Error = MergedLoggerError<L1::Error, L2::Error>;

    async fn log(&mut self, reading: SensorReading<BootTimestamp, D>) -> Result<bool, Self::Error> {
        let result_1 = self.logger_1.log(reading.clone()).await;
        let result_2 = self.logger_2.log(reading).await;

        MergedLoggerError::from_results(result_1, result_2).map(|logged| logged.0 || logged.1)
    }

    async fn log_unix_time(
        &mut self,
        log: super::delta_logger::UnixTimestampLog,
    ) -> Result<(), Self::Error> {
        let result_1 = self.logger_1.log_unix_time(log.clone()).await;
        let result_2 = self.logger_2.log_unix_time(log).await;

        MergedLoggerError::from_results(result_1, result_2).map(|_| ())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        let result_1 = self.logger_1.flush().await;
        let result_2 = self.logger_2.flush().await;

        MergedLoggerError::from_results(result_1, result_2).map(|_| ())
    }

    async fn into_inner(self) -> Result<(L1, L2), Self::Error> {
        Ok((self.logger_1, self.logger_2))
    }
}