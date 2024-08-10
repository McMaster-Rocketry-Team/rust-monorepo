use embassy_sync::{blocking_mutex::raw::RawMutex, signal::Signal};

use crate::common::zerocopy_channel::ZeroCopyChannel;

use super::{ProcessedSGReading, RawSGReadings};

pub struct SGGlobalStates<M: RawMutex> {
    pub(crate) raw_readings_channel: ZeroCopyChannel<M, RawSGReadings>,
    pub(crate) sg_enable_signal: Signal<M, bool>,
    pub(crate) processed_readings_channel: ZeroCopyChannel<M, [ProcessedSGReading; 4]>,
}

impl<M: RawMutex> SGGlobalStates<M> {
    pub const fn new_const() -> Self {
        Self {
            raw_readings_channel: ZeroCopyChannel::new(
                RawSGReadings::new_const(),
                RawSGReadings::new_const(),
            ),
            sg_enable_signal: Signal::new(),
            processed_readings_channel: ZeroCopyChannel::new(
                [
                    ProcessedSGReading::new_const(0),
                    ProcessedSGReading::new_const(1),
                    ProcessedSGReading::new_const(2),
                    ProcessedSGReading::new_const(3),
                ],
                [
                    ProcessedSGReading::new_const(0),
                    ProcessedSGReading::new_const(1),
                    ProcessedSGReading::new_const(2),
                    ProcessedSGReading::new_const(3),
                ],
            ),
        }
    }
}
