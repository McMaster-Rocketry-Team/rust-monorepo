use vlfs::{Crc, FileType, Flash, VLFSError, VLFS};

use crate::{
    common::{fixed_point::F64FixedPointFactory, sensor_reading::{SensorData, SensorReading}},
    driver::timestamp::TimestampType,
};

use super::ring_delta_logger::RingDeltaLogger;

pub struct TieredRingDeltaLoggerConfig {
    tier_1_seconds_per_segment: u32,
    tier_1_keep_seconds: u32,
    tier_2_seconds_per_segment: u32,
    tier_2_keep_seconds: u32,
}

pub struct TieredRingDeltaLogger<'a, TM, D, C, F, FF1, FF2>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF1: F64FixedPointFactory,
    FF2: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    delta_logger_1: RingDeltaLogger<'a, TM, D, C, F, FF1>,
    delta_logger_2: RingDeltaLogger<'a, TM, D, C, F, FF2>,
}

impl<'a, TM, D, C, F, FF1, FF2> TieredRingDeltaLogger<'a, TM, D, C, F, FF1, FF2>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF1: F64FixedPointFactory,
    FF2: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        tier_1_file_type: FileType,
        tier_2_file_type: FileType,
        config: &TieredRingDeltaLoggerConfig,
    ) -> Result<Self, VLFSError<F::Error>> {
        Ok(Self {
            delta_logger_1: RingDeltaLogger::new(
                fs,
                tier_1_file_type,
                config.tier_1_seconds_per_segment,
                config.tier_1_keep_seconds,
            )
            .await?,
            delta_logger_2: RingDeltaLogger::new(
                fs,
                tier_2_file_type,
                config.tier_2_seconds_per_segment,
                config.tier_2_keep_seconds,
            )
            .await?,
        })
    }

    pub async fn log(&mut self, value: SensorReading<TM, D>) -> Result<(), VLFSError<F::Error>> {
        let result_1 = self.delta_logger_1.log(value.clone()).await;
        let result_2 = self.delta_logger_2.log(value).await;
        result_1?;
        result_2?;
        Ok(())
    }

    pub async fn close(self) -> Result<(), VLFSError<F::Error>> {
        let result_1 = self.delta_logger_1.close().await;
        let result_2 = self.delta_logger_2.close().await;
        result_1?;
        result_2?;
        Ok(())
    }
}
