use super::global_states::SGGlobalStates;
use crate::driver::sg_adc::{RawSGReadingsTrait, SGAdc};
use embassy_sync::blocking_mutex::raw::RawMutex;

pub async fn sg_high_prio_main<T: RawSGReadingsTrait>(
    states: &SGGlobalStates<impl RawMutex, T>,
    mut sg_adc: impl SGAdc<T>,
) {
    let mut raw_readings_sender = states.raw_readings_channel.sender();
    loop {
        match sg_adc.read(&mut raw_readings_sender).await {
            Ok(_) => {}
            Err(e) => {
                log_error!("Error reading strain gauges: {:?}", e);
            }
        };
    }
}
