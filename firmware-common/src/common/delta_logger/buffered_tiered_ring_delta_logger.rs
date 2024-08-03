use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubBehavior, PubSubChannel},
    signal::Signal,
};
use futures::join;
use vlfs::{Crc, Flash};

use crate::{
    common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
    },
    driver::timestamp::BootTimestamp,
    try_or_warn, Clock, Delay,
};

use super::{delta_logger::UnixTimeLog, prelude::TieredRingDeltaLogger};

pub struct BufferedTieredRingDeltaLogger<D, const CAP: usize>
where
    D: SensorData,
    [(); size_of::<D>() + 10]:,
{
    channel: PubSubChannel<
        NoopRawMutex,
        either::Either<SensorReading<BootTimestamp, D>, UnixTimeLog>,
        CAP,
        1,
        1,
    >,
    close_signal: Signal<NoopRawMutex, ()>,
}

impl<D, const CAP: usize> BufferedTieredRingDeltaLogger<D, CAP>
where
    D: SensorData,
    [(); size_of::<D>() + 10]:,
{
    pub fn new() -> Self {
        Self {
            channel: PubSubChannel::new(),
            close_signal: Signal::new(),
        }
    }

    pub fn log(&self, value: SensorReading<BootTimestamp, D>) {
        self.channel.publish_immediate(either::Either::Left(value));
    }

    pub fn log_unix_time(&mut self, log: UnixTimeLog) {
        self.channel.publish_immediate(either::Either::Right(log));
    }

    pub fn close(&self) {
        self.close_signal.signal(());
    }

    pub async fn run<'a, FF1, FF2, C, F, DL, CL>(
        &self,
        _ff1: FF1,
        _ff2: FF2,
        logger: TieredRingDeltaLogger<'a, D, C, F, FF1, FF2, DL, CL>,
    ) where
        C: Crc,
        F: Flash,
        F::Error: defmt::Format,
        FF1: F64FixedPointFactory,
        FF2: F64FixedPointFactory,
        DL: Delay,
        CL: Clock,
    {
        logger.log_stats();
        log_info!("Buffer size: {}kb", size_of_val(&self.channel) / 1024);
        log_info!("Buffer duration: {}s", FF1::min() * CAP as f64 / 1000.0);
        let sub_fut = async {
            let mut sub = self.channel.subscriber().unwrap();
            loop {
                match select(sub.next_message_pure(), self.close_signal.wait()).await {
                    Either::First(either::Either::Left(value)) => {
                        try_or_warn!(logger.log(value).await);
                    }
                    Either::First(either::Either::Right(log)) => {
                        try_or_warn!(logger.log_unix_time(log).await);
                    }
                    Either::Second(_) => {
                        logger.close();
                        break;
                    }
                }
            }
        };

        let logger_fut = logger.run();

        join!(sub_fut, logger_fut);
    }
}
