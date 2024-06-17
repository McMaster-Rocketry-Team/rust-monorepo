use crate::{claim_devices, common::device_manager::prelude::*};

pub enum SelfTestResult {
    Ok,
    PartialFailed,
    Failed,
}

pub async fn self_test(device_manager: device_manager_type!()) -> SelfTestResult {
    claim_devices!(device_manager, imu, barometer, meg);
    // reset all devices
    imu.reset().await.unwrap();
    barometer.reset().await.unwrap();
    meg.reset().await.unwrap();

    let imu = imu.read().await;
    let baro = barometer.read().await;
    let meg = meg.read().await;

    log_info!("Self test: {:?} {:?} {:?}", imu, baro, meg);
    imu.is_ok() && baro.is_ok() && meg.is_ok();
    SelfTestResult::Ok // FIXME
}
