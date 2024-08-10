use core::fmt::Debug;

use embassy_sync::blocking_mutex::raw::RawMutex;

use crate::common::zerocopy_channel::ZeroCopyChannel;

use super::{global_states::SGGlobalStates, RawSGReadings};

pub trait SGAdc {
    type Error: defmt::Format + Debug;

    async fn read<M: RawMutex>(
        &mut self,
        channel: &ZeroCopyChannel<M, RawSGReadings>,
    ) -> Result<(), Self::Error>;
}

pub async fn high_prio_main(state: &SGGlobalStates<impl RawMutex>, mut sg_adc: impl SGAdc) {
    loop {
        match sg_adc.read(&state.raw_readings_channel).await {
            Ok(_) => {}
            Err(e) => {
                log_error!("Error reading strain gauges: {:?}", e);
            }
        };
    }
}
