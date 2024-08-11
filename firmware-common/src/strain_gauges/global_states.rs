use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;

use crate::common::zerocopy_channel::ZeroCopyChannel;
use crate::driver::sg_adc::RawSGReadingsTrait;

use super::ProcessedSGReading;

pub struct SGGlobalStates<M: RawMutex, T: RawSGReadingsTrait> {
    pub(crate) raw_readings_channel: ZeroCopyChannel<M, T,20>,
    pub(crate) processed_readings_channel:  ZeroCopyChannel<M, ProcessedSGReading,4>,
    pub(crate) led_state: BlockingMutex<M, RefCell<SGLEDState>>,
}

impl<M: RawMutex, T: RawSGReadingsTrait> SGGlobalStates<M, T> {
    pub fn new() -> Self {
        Self {
            raw_readings_channel: ZeroCopyChannel::new(),
            processed_readings_channel: ZeroCopyChannel::new(),
            led_state: BlockingMutex::new(RefCell::new(SGLEDState {
                can_bus_error: false,
                sg_adc_error: false,
            })),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct SGLEDState {
    pub(crate) can_bus_error: bool,
    pub(crate) sg_adc_error: bool,
}
