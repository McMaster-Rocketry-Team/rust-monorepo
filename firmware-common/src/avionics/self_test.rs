use crate::{claim_devices, common::device_manager::prelude::*};
use defmt::unwrap;

pub async fn self_test(device_manager: device_manager_type!()) -> bool {
    claim_devices!(device_manager, imu, barometer, meg);
    unwrap!(imu.reset().await);
    unwrap!(barometer.reset().await);
    unwrap!(meg.reset().await);

    let imu = imu.read().await;
    let baro = barometer.read().await;
    let meg = meg.read().await;

    log_info!("Self test: {:?} {:?} {:?}", imu, baro, meg);
    imu.is_ok() && baro.is_ok() && meg.is_ok()
}
