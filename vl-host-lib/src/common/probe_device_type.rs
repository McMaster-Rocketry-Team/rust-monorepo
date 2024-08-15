use std::time::Duration;

use crate::create_serial;
use anyhow::anyhow;
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{common::console::DeviceType, vl_rpc};
use tokio::time::sleep;

struct Delay;

impl DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        sleep(Duration::from_nanos(ns as u64)).await;
    }
}

pub async fn probe_device_type(serial_port_name: String) -> Result<DeviceType> {
    let mut serial = create_serial(serial_port_name)?;
    let mut client = vl_rpc::RpcClient::new(&mut serial, Delay);
    client.reset().await.map_err(|_| anyhow!("reset error"))?;

    client
        .get_device_type()
        .await
        .map(|response| response.device_type)
        .map_err(|_| anyhow!("get_device_type error"))
}
