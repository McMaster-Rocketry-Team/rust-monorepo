use futures::join;
use vlfs::{Crc, FileType, Flash, VLFSError, VLFS};

use crate::{
    common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    driver::timestamp::TimestampType,
    Clock, Delay,
};

use super::ring_delta_logger::{RingDeltaLogger, RingDeltaLoggerConfig};

pub struct TieredRingDeltaLogger<'a, TM, D, C, F, FF1, FF2, DL, CL>
where
    TM: TimestampType,
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
    delta_logger_1: RingDeltaLogger<'a, TM, D, C, F, FF1, DL, CL>,
    delta_logger_2: RingDeltaLogger<'a, TM, D, C, F, FF2, DL, CL>,
}

impl<'a, TM, D, C, F, FF1, FF2, DL, CL> TieredRingDeltaLogger<'a, TM, D, C, F, FF1, FF2, DL, CL>
where
    TM: TimestampType,
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
        tier_1_file_type: FileType,
        tier_1_config: RingDeltaLoggerConfig,
        tier_2_file_type: FileType,
        tier_2_config: RingDeltaLoggerConfig,
        delay: DL,
        clock: CL,
    ) -> Result<Self, VLFSError<F::Error>> {
        Ok(Self {
            delta_logger_1: RingDeltaLogger::new(
                fs,
                tier_1_file_type,
                delay.clone(),
                clock.clone(),
                tier_1_config,
            )
            .await?,
            delta_logger_2: RingDeltaLogger::new(
                fs,
                tier_2_file_type,
                delay.clone(),
                clock.clone(),
                tier_2_config,
            )
            .await?,
        })
    }

    pub async fn log(&self, value: SensorReading<TM, D>) -> Result<(), VLFSError<F::Error>> {
        let result_1 = self.delta_logger_1.log(value.clone()).await;
        let result_2 = self.delta_logger_2.log(value).await;
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
}
