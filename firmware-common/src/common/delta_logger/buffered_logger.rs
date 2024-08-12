use core::convert::Infallible;
use core::marker::PhantomData;

use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::Mutex,
    pubsub::{PubSubBehavior, PubSubChannel},
    signal::Signal,
};

use crate::{
    common::{rpc_channel::{RpcChannel, RpcChannelClient, RpcChannelServer}, sensor_reading::{SensorData, SensorReading}},
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
    flush_rpc: RpcChannel<NoopRawMutex, (), ()>
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
            flush_rpc: RpcChannel::new(),
        }
    }

    pub fn get_logger_runner(
        &self,
    ) -> (
        BufferedLogger<'_, D, I, L, CAP>,
        BufferedLoggerRunner<'_, D, I, L, CAP>,
    ) {
        
        (
            BufferedLogger { state: self, flush_rpc_client: self.flush_rpc.client()},
            BufferedLoggerRunner { state: self, flush_rpc_server: self.flush_rpc.server() },
        )
    }
}

pub struct BufferedLogger<'a, D, I, L, const CAP: usize>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    state: &'a BufferedLoggerState<D, I, L, CAP>,
    flush_rpc_client: RpcChannelClient<'a, NoopRawMutex, (),()>
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
        self.flush_rpc_client.call(()).await;
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
    flush_rpc_server: RpcChannelServer<'a, NoopRawMutex, (),()>
}

impl<'a, D, I, L, const CAP: usize> BufferedLoggerRunner<'a, D, I, L, CAP>
where
    D: SensorData,
    L: DeltaLoggerTrait<D, I>,
{
    async fn log(logger: &mut L, log:either::Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>){
        match log{
            either::Either::Left(value) => {
                try_or_warn!(logger.log(value).await);
            },
            either::Either::Right(log) => {
                try_or_warn!(logger.log_unix_time(log).await);
            },
        }
    }

    pub async fn run(&mut self) -> Result<(), L::Error> {
        let mut logger = self.state.logger.lock().await;
        let logger = logger.as_mut().unwrap();

        // log_info!("Buffer size: {}kb", size_of_val(&self.channel) / 1024);
        // log_info!("Buffer duration: {}s", FF::min() * CAP as f64 / 1000.0);
        let mut sub = self.state.channel.subscriber().unwrap();
        loop {
            match select3(sub.next_message_pure(), self.flush_rpc_server.get_request(), self.state.stop_signal.wait()).await{
                Either3::First(log) =>{
                    Self::log(logger, log).await;
                },
                Either3::Second(_) => {
                    // flush
                    while let Some(message) = sub.try_next_message_pure() {
                        Self::log(logger, message).await;
                    }
                    try_or_warn!(logger.flush().await);
                },
                Either3::Third(_) => {
                    // stop
                    while let Some(message) = sub.try_next_message_pure() {
                        Self::log(logger, message).await;
                    }
                    try_or_warn!(logger.flush().await);
                    break;
                },
            }
        }

        Ok(())
    }
}
