use core::convert::Infallible;
use core::marker::PhantomData;

use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
    signal::Signal,
};

use crate::{
    common::sensor_reading::{SensorData, SensorReading},
    driver::timestamp::BootTimestamp,
    try_or_warn,
};

use super::{
    delta_logger::UnixTimestampLog, delta_logger_trait::BackgroundRunDeltaLoggerTrait,
    prelude::DeltaLoggerTrait,
};

pub struct BufferedLogger<D, I, L, const CAP: usize>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    phantom: PhantomData<I>,
    logger: Mutex<NoopRawMutex, Option<L>>,
    channel: PubSubChannel<
        NoopRawMutex,
        either::Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>,
        CAP,
        1,
        1,
    >,
    stop_signal: Signal<NoopRawMutex, ()>,
}

impl<D, I, L, const CAP: usize> DeltaLoggerTrait<D, L> for &BufferedLogger<D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    type Error = Infallible;

    async fn log(&mut self, reading: SensorReading<BootTimestamp, D>) -> Result<bool, Self::Error> {
        self.channel
            .publish_immediate(either::Either::Left(reading));
        Ok(true)
    }

    async fn log_unix_time(&mut self, log: UnixTimestampLog) -> Result<(), Self::Error> {
        self.channel.publish_immediate(either::Either::Right(log));
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // noop
        Ok(())
    }

    async fn into_inner(self) -> Result<L, Self::Error> {
        self.stop_signal.signal(());
        let mut logger = self.logger.lock().await;
        Ok(logger.take().unwrap())
    }
}

impl<D, I, L, const CAP: usize> BackgroundRunDeltaLoggerTrait<D, L>
    for &BufferedLogger<D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    async fn run(&mut self) -> Result<(), Self::Error> {
        let mut logger = self.logger.lock().await;
        let logger = logger.as_mut().unwrap();

        // log_info!("Buffer size: {}kb", size_of_val(&self.channel) / 1024);
        // log_info!("Buffer duration: {}s", FF::min() * CAP as f64 / 1000.0);
        let mut sub = self.channel.subscriber().unwrap();
        loop {
            match select(sub.next_message_pure(), self.stop_signal.wait()).await {
                Either::First(either::Either::Left(value)) => {
                    try_or_warn!(logger.log(value).await);
                }
                Either::First(either::Either::Right(log)) => {
                    try_or_warn!(logger.log_unix_time(log).await);
                }
                Either::Second(_) => {
                    try_or_warn!(logger.flush().await);
                    break;
                }
            }
        }

        Ok(())
    }
}
