use super::global_states::SGGlobalStates;
use crate::driver::sg_adc::SGAdc;
use embassy_sync::blocking_mutex::raw::RawMutex;

pub async fn sg_high_prio_main(state: &SGGlobalStates<impl RawMutex>, mut sg_adc: impl SGAdc) {
    let mut sender = state.raw_readings_channel.sender();
    loop {
        match sg_adc.read(&mut sender).await {
            Ok(_) => {}
            Err(e) => {
                log_error!("Error reading strain gauges: {:?}", e);
            }
        };
    }
}
