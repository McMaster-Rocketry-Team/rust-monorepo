use super::global_states::SGGlobalStates;
use crate::driver::sg_adc::{RawSGReadingsTrait, SGAdc};
use embassy_sync::blocking_mutex::raw::RawMutex;

pub async fn sg_high_prio_main<T: RawSGReadingsTrait>(
    states: &SGGlobalStates<impl RawMutex, T>,
    mut sg_adc: impl SGAdc<T>,
)->! {
    log_info!("Starting sg_high_prio_main");
    let mut raw_readings_sender = states.raw_readings_channel.sender();
    loop {
        match sg_adc.read(&mut raw_readings_sender).await {
            Ok(_) => {
                states.error_states.lock(|led_state| {
                    led_state.borrow_mut().sg_adc_error = false;
                });
            }
            Err(e) => {
                log_error!("Error reading strain gauges: {:?}", e);
                states.error_states.lock(|led_state| {
                    led_state.borrow_mut().sg_adc_error = true;
                });
            }
        };
    }
}
