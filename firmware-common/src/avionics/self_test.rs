use crate::{common::device_manager::prelude::*, claim_devices};
use defmt::unwrap;

pub async fn self_test(device_manager: device_manager_type!()) -> bool {
    claim_devices!(device_manager, imu, barometer, meg);
    unwrap!(imu.wait_for_power_on().await);
    unwrap!(imu.reset().await);
    unwrap!(barometer.reset().await);
    unwrap!(meg.reset().await);

    let imu_ok = imu.read().await.is_ok();
    let baro_ok = barometer.read().await.is_ok();
    let meg_ok = meg.read().await.is_ok();
    imu_ok && baro_ok && meg_ok
}
