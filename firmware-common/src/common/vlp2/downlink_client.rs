use crate::{
    common::{config_structs::LoraConfig, unix_clock::UnixClock},
    Clock,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embedded_hal_async::delay::DelayNs;
use lora_phy::{
    mod_params::{PacketStatus, RadioError},
    mod_traits::RadioKind,
    LoRa, RxMode,
};

use super::{
    lora_phy::LoraPhy,
    packet::{AckPacket, VLPDownlinkPacket, VLPUplinkPacket, MAX_VLP_PACKET_SIZE},
    packet_builder::VLPPacketBuilder,
};

// VLP client running on the GCM
pub struct VLPDownlinkClient<'a, 'b, 'c, CL: Clock, DL: DelayNs + Copy>
where
    'a: 'b,
{
    lora_config: &'a LoraConfig,
    packet_builder: VLPPacketBuilder<'b, 'c, CL>,
    tx_signal: Signal<NoopRawMutex, VLPUplinkPacket>,
    rx_signal: Signal<NoopRawMutex, (VLPDownlinkPacket, PacketStatus)>,
    send_success_signal: Signal<NoopRawMutex, Option<PacketStatus>>,
    delay: DL,
}

impl<'a, 'b, 'c, CL: Clock, DL: DelayNs + Copy> VLPDownlinkClient<'a, 'b, 'c, CL, DL>
where
    'a: 'b,
{
    pub fn new(
        lora_config: &'a LoraConfig,
        unix_clock: UnixClock<'a, CL>,
        delay: DL,
        key: &'c [u8; 32],
    ) -> Self {
        VLPDownlinkClient {
            packet_builder: VLPPacketBuilder::new(unix_clock, lora_config.into(), key),
            lora_config,
            tx_signal: Signal::new(),
            rx_signal: Signal::new(),
            send_success_signal: Signal::new(),
            delay,
        }
    }

    /// Calling send multiple times concurrently is not supported
    /// Returns the packet status of the ack message
    pub async fn send(&self, packet: VLPUplinkPacket) -> Option<PacketStatus> {
        self.tx_signal.signal(packet);
        self.send_success_signal.wait().await
    }

    pub async fn wait_receive(&self) -> (VLPDownlinkPacket, PacketStatus) {
        self.rx_signal.wait().await
    }

    pub async fn run(&self, lora: &mut LoRa<impl RadioKind, impl DelayNs>) {
        let mut delay = self.delay;
        let mut lora = LoraPhy::new(lora, self.lora_config);
        let mut buffer = [0; MAX_VLP_PACKET_SIZE];

        let deserializer = |buffer: &[u8]| self.packet_builder.deserialize_downlink(buffer);

        loop {
            let result: Result<(), RadioError> = try {
                match lora
                    .rx(RxMode::Single(1000), &mut buffer, deserializer)
                    .await
                {
                    Ok(Some((packet, packet_status))) => {
                        self.rx_signal.signal((packet, packet_status));
                    }
                    Err(RadioError::ReceiveTimeout) | Ok(None) => {}
                    Err(e) => {
                        Err(e)?;
                    }
                }

                if self.tx_signal.signaled() {
                    let tx_packet = self.tx_signal.wait().await;
                    log_info!("Sending message: {:?}", tx_packet);

                    let mut ack_packet_status: Option<PacketStatus> = None;
                    for i in 0..5 {
                        let tx_packet_serialized = self
                            .packet_builder
                            .serialize_uplink(&mut buffer, &tx_packet);
                        lora.tx(tx_packet_serialized).await?;

                        match lora
                            .rx(RxMode::Single(100), &mut buffer, deserializer)
                            .await
                        {
                            Ok(Some((packet, packet_status))) => {
                                if matches!(packet, VLPDownlinkPacket::AckPacket(AckPacket { .. }))
                                {
                                    log_info!(
                                        "Ack received: rssi: {}, snr: {}",
                                        packet_status.rssi,
                                        packet_status.snr
                                    );
                                    ack_packet_status = Some(packet_status);
                                    break;
                                } else {
                                    log_warn!("Expected AckPacket, but received {:?}", packet);
                                }
                            }
                            Err(RadioError::ReceiveTimeout) | Ok(None) => {}
                            Err(e) => {
                                Err(e)?;
                            }
                        }

                        log_warn!("Ack not received, retrying {}", i + 1);
                    }
                    if ack_packet_status.is_some() {
                        log_info!("Message sent successfully");
                    } else {
                        log_warn!("Failed to send message");
                    }
                    self.send_success_signal.signal(ack_packet_status);
                }
            };
            if let Err(e) = result {
                log_error!("Error in VLP downlink client: {:?}", e);
                delay.delay_ms(1000).await;
            }
        }
    }
}
