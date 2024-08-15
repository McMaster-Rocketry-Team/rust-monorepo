use std::time::Duration;

use crate::create_serial;
use anyhow::anyhow;
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::sg_rpc;
use firmware_common::{common::console::DeviceType, vl_rpc};
use tokio::time::sleep;

struct Delay;

impl DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        sleep(Duration::from_nanos(ns as u64)).await;
    }
}

pub async fn probe_device_type(serial_port_name: String) -> Result<String> {
    let mut serial = create_serial(serial_port_name)?;
    let mut client = vl_rpc::RpcClient::new(&mut serial, Delay);
    client.reset().await.map_err(|_| anyhow!("reset error"))?;

    let device_type = client
        .get_device_type()
        .await
        .map(|response| response.device_type)
        .map_err(|_| anyhow!("get_device_type error"))?;

    Ok(match device_type {
        DeviceType::VoidLake => {
            let who_am_i = client
                .who_am_i()
                .await
                .map_err(|_| anyhow!("who_am_i error"))?;
            format!(
                "{:?}, {}, SN: {:02X?}",
                device_type,
                who_am_i
                    .name
                    .map_or("".into(), |s| String::from(s.as_str())),
                who_am_i.serial_number
            )
        }
        DeviceType::OZYS => {
            drop(client);
            let mut client = sg_rpc::RpcClient::new(&mut serial, Delay);
            let who_am_i = client
                .who_am_i()
                .await
                .map_err(|_| anyhow!("who_am_i error"))?;
            format!("{:?}, SN: {:02X?}", device_type, who_am_i.serial_number)
        }
    })
}
