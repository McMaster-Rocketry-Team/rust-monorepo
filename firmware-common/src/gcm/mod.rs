use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};
use futures::join;
use lora_phy::mod_params::PacketStatus;
use vlfs::{Crc, Flash};

use crate::{
    claim_devices,
    common::{
        config_structs::{DeviceConfig, DeviceModeConfig},
        device_manager::prelude::*,
        vlp2::{
            downlink_client::VLPDownlinkClient,
            packet::{VLPDownlinkPacket, VLPUplinkPacket},
        },
    },
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator},
};

#[inline(never)]
pub async fn gcm_main(
    device_manager: device_manager_type!(),
    services: system_services_type!(),
    config: DeviceConfig,
    uplink_package_receiver: Receiver<'_, impl RawMutex, VLPUplinkPacket, 1>,
    downlink_package_sender: Sender<'_, impl RawMutex, (VLPDownlinkPacket, PacketStatus), 1>,
) {
    let lora_key = if let DeviceModeConfig::GCM { lora_key } = &config.mode {
        lora_key
    } else {
        log_unreachable!()
    };

    claim_devices!(device_manager, lora, indicators);

    let indicators_fut = indicators.run([], [], [250, 250]);
    let wait_gps_fut = services.unix_clock.wait_until_ready();
    select(indicators_fut, wait_gps_fut).await;

    let vlp_client =
        VLPDownlinkClient::new(&config.lora, services.unix_clock, services.delay, lora_key);
    let vlp_client_fut = vlp_client.run(&mut lora);

    let vlp_send_fut = async {
        loop {
            let package = uplink_package_receiver.receive().await;
            vlp_client.send(package);
        }
    };

    let vlp_receive_fut = async {
        loop {
            let received = vlp_client.wait_receive().await;
            downlink_package_sender.send(received).await;
        }
    };

    #[allow(unreachable_code)]
    {
        join!(vlp_client_fut, vlp_send_fut, vlp_receive_fut,);
    }
}
