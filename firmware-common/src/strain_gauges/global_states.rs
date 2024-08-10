use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;

use crate::common::zerocopy_channel::ZeroCopyChannel;
use crate::driver::sg_adc::RawSGReadings;

use super::ProcessedSGReading;

pub struct SGGlobalStates<M: RawMutex> {
    pub(crate) raw_readings_channel: ZeroCopyChannel<M, RawSGReadings>,
    pub(crate) processed_readings_channel: ZeroCopyChannel<M, [ProcessedSGReading; 4]>,
    pub(crate) led_state: BlockingMutex<M, RefCell<SGLEDState>>,
}

impl<M: RawMutex> SGGlobalStates<M> {
    pub const fn new_const() -> Self {
        Self {
            raw_readings_channel: ZeroCopyChannel::new(
                RawSGReadings::new_const(),
                RawSGReadings::new_const(),
            ),
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
