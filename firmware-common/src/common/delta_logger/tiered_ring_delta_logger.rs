use futures::join;
use vlfs::{Crc, Flash, VLFSError, VLFS};

use crate::{
    common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    driver::timestamp::BootTimestamp,
    Clock, Delay,
};

use super::{
    delta_logger::UnixTimestampLog,
    ring_delta_logger::{RingDeltaLogger, RingDeltaLoggerConfig},
};

pub struct TieredRingDeltaLogger<'a, D, C, F, FF1, FF2, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF1: F64FixedPointFactory,
    FF2: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    delta_logger_1: RingDeltaLogger<'a, D, C, F, FF1, DL, CL>,
    delta_logger_2: RingDeltaLogger<'a, D, C, F, FF2, DL, CL>,
}

impl<'a, D, C, F, FF1, FF2, DL, CL> TieredRingDeltaLogger<'a, D, C, F, FF1, FF2, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF1: F64FixedPointFactory,
    FF2: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        configs: (RingDeltaLoggerConfig, RingDeltaLoggerConfig),
        delay: DL,
        clock: CL,
    ) -> Result<Self, VLFSError<F::Error>> {
        Ok(Self {
            delta_logger_1: RingDeltaLogger::new(fs, delay.clone(), clock.clone(), configs.0)
                .await?,
            delta_logger_2: RingDeltaLogger::new(fs, delay.clone(), clock.clone(), configs.1)
                .await?,
        })
    }

    pub async fn log(
        &self,
        value: SensorReading<BootTimestamp, D>,
    ) -> Result<(), VLFSError<F::Error>> {
        let result_1 = self.delta_logger_1.log(value.clone()).await;
        let result_2 = self.delta_logger_2.log(value).await;
        result_1?;
        result_2?;
        Ok(())
    }

    pub async fn log_unix_time(&self, log: UnixTimestampLog) -> Result<(), VLFSError<F::Error>> {
        let result_1 = self.delta_logger_1.log_unix_time(log.clone()).await;
        let result_2 = self.delta_logger_2.log_unix_time(log.clone()).await;
        result_1?;
        result_2?;
        Ok(())
    }

    pub fn close(&self) {
        self.delta_logger_1.close();
        self.delta_logger_2.close();
    }

    pub async fn run(&self) {
        let logger_1_fut = self.delta_logger_1.run();
        let logger_2_fut = self.delta_logger_2.run();

        join!(logger_1_fut, logger_2_fut);
    }

    pub fn log_stats(&self) {
        log_info!("Tier 1:");
        self.delta_logger_1.log_stats();
        log_info!("Tier 2:");
        self.delta_logger_2.log_stats();
    }
}
