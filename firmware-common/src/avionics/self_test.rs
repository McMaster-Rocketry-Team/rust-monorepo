use crate::{claim_devices, common::device_manager::prelude::*};

pub enum SelfTestResult {
    Ok,
    PartialFailed,
    Failed,
}

pub async fn self_test(device_manager: device_manager_type!()) -> SelfTestResult {
    claim_devices!(device_manager, low_g_imu, high_g_imu, barometer, mag);
    // reset all devices
    low_g_imu.reset().await.unwrap();
    high_g_imu.reset().await.unwrap();
    barometer.reset().await.unwrap();
    mag.reset().await.unwrap();

    let low_g_imu = low_g_imu.read().await;
    let high_g_imu = high_g_imu.read().await;
    let baro = barometer.read().await;
    let mag = mag.read().await;

    log_info!(
        "Self test: {:?} {:?} {:?} {:?}",
        low_g_imu,
        high_g_imu,
        baro,
        mag
    );
    let critical_ok = low_g_imu.is_ok() && high_g_imu.is_ok() && baro.is_ok();

    if !critical_ok {
        return SelfTestResult::Failed;
    }

    let aux_ok = mag.is_ok();
    if aux_ok {
        return SelfTestResult::Ok;
    } else {
        return SelfTestResult::PartialFailed;
    }
}
