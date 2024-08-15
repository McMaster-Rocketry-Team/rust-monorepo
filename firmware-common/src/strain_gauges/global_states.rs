use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::pubsub::PubSubChannel;

use crate::common::zerocopy_channel::ZeroCopyChannel;
use crate::driver::sg_adc::RawSGReadingsTrait;

use super::ProcessedSGReading;

pub struct SGGlobalStates<M: RawMutex, T: RawSGReadingsTrait> {
    pub(crate) raw_readings_channel: ZeroCopyChannel<M, T, 20>,
    pub(crate) processed_readings_channel: ZeroCopyChannel<M, ProcessedSGReading, 8>,
    pub(crate) realtime_sample_pubsub: PubSubChannel<M, [f32; 4], 4, 1, 1>,
    pub(crate) error_states: BlockingMutex<M, RefCell<SGLEDState>>,
}

impl<M: RawMutex, T: RawSGReadingsTrait> SGGlobalStates<M, T> {
    pub fn new() -> Self {
        Self {
            raw_readings_channel: ZeroCopyChannel::new(),
            processed_readings_channel: ZeroCopyChannel::new(),
            realtime_sample_pubsub: PubSubChannel::new(),
            error_states: BlockingMutex::new(RefCell::new(SGLEDState {
                can_bus_error: false,
                sg_adc_error: false,
                usb_connected: false,
            })),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct SGLEDState {
    pub(crate) can_bus_error: bool,
    pub(crate) sg_adc_error: bool,
    pub(crate) usb_connected: bool,
}
