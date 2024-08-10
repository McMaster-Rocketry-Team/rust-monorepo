use super::global_states::SGGlobalStates;
use crate::driver::sg_adc::SGAdc;
use embassy_sync::blocking_mutex::raw::RawMutex;

pub async fn high_prio_main(state: &SGGlobalStates<impl RawMutex>, mut sg_adc: impl SGAdc) {
    if let Err(e) = sg_adc.reset().await {
        log_error!("Error resetting strain gauges: {:?}", e);
        return;
    }

    loop {
        match sg_adc.read(&state.raw_readings_channel).await {
            Ok(_) => {}
            Err(e) => {
                log_error!("Error reading strain gauges: {:?}", e);
            }
        };
    }
}
