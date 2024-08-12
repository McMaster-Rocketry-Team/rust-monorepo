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

use super::{delta_logger::UnixTimestampLog, prelude::DeltaLoggerTrait};

pub struct BufferedLoggerState<D, I, L, const CAP: usize>
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

impl<D, I, L, const CAP: usize> BufferedLoggerState<D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    pub fn new(logger: L) -> Self {
        Self {
            phantom: PhantomData,
            logger: Mutex::new(Some(logger)),
            channel: PubSubChannel::new(),
            stop_signal: Signal::new(),
        }
    }

    pub fn get_logger_runner(
        &self,
    ) -> (
        BufferedLogger<'_, D, I, L, CAP>,
        BufferedLoggerRunner<'_, D, I, L, CAP>,
    ) {
        (
            BufferedLogger { state: self },
            BufferedLoggerRunner { state: self },
        )
    }
}

pub struct BufferedLogger<'a, D, I, L, const CAP: usize>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    state: &'a BufferedLoggerState<D, I, L, CAP>,
}

impl<'a, D, I, L, const CAP: usize> DeltaLoggerTrait<D, L> for BufferedLogger<'a, D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    type Error = Infallible;

    async fn log(&mut self, reading: SensorReading<BootTimestamp, D>) -> Result<bool, Self::Error> {
        self.state
            .channel
            .publish_immediate(either::Either::Left(reading));
        Ok(true)
    }

    async fn log_unix_time(&mut self, log: UnixTimestampLog) -> Result<(), Self::Error> {
        self.state
            .channel
            .publish_immediate(either::Either::Right(log));
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // noop
        Ok(())
    }

    async fn into_inner(self) -> Result<L, Self::Error> {
        self.state.stop_signal.signal(());
        let mut logger = self.state.logger.lock().await;
        Ok(logger.take().unwrap())
    }
}

pub struct BufferedLoggerRunner<'a, D, I, L, const CAP: usize>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    state: &'a BufferedLoggerState<D, I, L, CAP>,
}

impl<'a, D, I, L, const CAP: usize> BufferedLoggerRunner<'a, D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    pub async fn run(&mut self) -> Result<(), L::Error> {
        let mut logger = self.state.logger.lock().await;
        let logger = logger.as_mut().unwrap();

        // log_info!("Buffer size: {}kb", size_of_val(&self.channel) / 1024);
        // log_info!("Buffer duration: {}s", FF::min() * CAP as f64 / 1000.0);
        let mut sub = self.state.channel.subscriber().unwrap();
        loop {
            match select(sub.next_message_pure(), self.state.stop_signal.wait()).await {
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
