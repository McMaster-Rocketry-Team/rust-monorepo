use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use futures::join;
use lora_phy::mod_params::PacketStatus;
use vlfs::{Crc, Flash};

use crate::{
    claim_devices,
    common::{
        device_config::DeviceConfig,
        vl_device_manager::prelude::*,
        rpc_channel::RpcChannelServer,
        vlp::{
            downlink_client::VLPDownlinkClient,
            packet::{VLPDownlinkPacket, VLPUplinkPacket},
        },
    },
    vl_device_manager_type,
    driver::indicator::Indicator,
};

#[inline(never)]
pub async fn gcm_main(
    device_manager: vl_device_manager_type!(),
    services: system_services_type!(),
    config: &DeviceConfig,
    downlink_package_sender: Sender<'_, NoopRawMutex, (VLPDownlinkPacket, PacketStatus), 1>,
    mut send_uplink_packet_rpc_server: RpcChannelServer<
        '_,
        NoopRawMutex,
        VLPUplinkPacket,
        Option<PacketStatus>,
    >,
) {
    claim_devices!(device_manager, lora, indicators);

    let indicators_fut = indicators.run([], [], [250, 250]);
    let wait_gps_fut = services.unix_clock.wait_until_ready();
    select(indicators_fut, wait_gps_fut).await;
    let indictors_fut = indicators.run([], [50, 950], []);

    let vlp_client = VLPDownlinkClient::new();
    let vlp_client_fut = vlp_client.run(
        services.delay(),
        lora.as_mut().unwrap(),
        &config.lora,
        services.unix_clock(),
        &config.lora_key,
    );

    let vlp_send_fut = async {
        loop {
            let packet = send_uplink_packet_rpc_server.get_request().await;
            let response = vlp_client.send(packet).await;
            send_uplink_packet_rpc_server.send_response(response).await;
        }
    };

    let vlp_receive_fut = async {
        loop {
            let received = vlp_client.wait_receive().await;
            downlink_package_sender.try_send(received).ok();
        }
    };

    #[allow(unreachable_code)]
    {
        join!(vlp_client_fut, vlp_send_fut, vlp_receive_fut, indictors_fut);
    }
}
