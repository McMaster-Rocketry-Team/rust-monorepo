use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embedded_hal_async::delay::DelayNs;
use lora_phy::{
    mod_params::{DutyCycleParams, PacketStatus, RadioError},
    mod_traits::RadioKind,
    LoRa, RxMode,
};

use crate::{
    common::{config_structs::LoraConfig, unix_clock::UnixClock},
    Clock,
};

use super::{
    lora_phy::LoraPhy,
    packet::{
        AckPacket, LowPowerModePacket, VLPDownlinkPacket, VLPUplinkPacket, MAX_VLP_PACKET_SIZE,
    },
    packet_builder::VLPPacketBuilder,
};

// VLP client running on the rocket
pub struct VLPUplinkClient<'a, 'b, 'c, CL: Clock, DL: DelayNs + Copy>
where
    'a: 'b,
{
    lora_config: &'a LoraConfig,
    packet_builder: VLPPacketBuilder<'b, 'c, CL>,
    unix_clock: UnixClock<'a, CL>,
    tx_signal: Signal<NoopRawMutex, VLPDownlinkPacket>,
    rx_signal: Signal<NoopRawMutex, (VLPUplinkPacket, PacketStatus)>,
    delay: DL,
}

impl<'a, 'b, 'c, CL: Clock, DL: DelayNs + Copy> VLPUplinkClient<'a, 'b, 'c, CL, DL>
where
    'a: 'b,
{
    pub fn new(
        lora_config: &'a LoraConfig,
        unix_clock: UnixClock<'a, CL>,
        delay: DL,
        key: &'c [u8; 32],
    ) -> Self {
        VLPUplinkClient {
            packet_builder: VLPPacketBuilder::new(unix_clock, lora_config.into(), key),
            unix_clock,
            lora_config,
            tx_signal: Signal::new(),
            rx_signal: Signal::new(),
            delay,
        }
    }

    pub fn send(&self, packet: VLPDownlinkPacket) {
        self.tx_signal.signal(packet);
    }

    pub async fn wait_receive(&self) -> (VLPUplinkPacket, PacketStatus) {
        self.rx_signal.wait().await
    }

    pub async fn run(&self, lora: &mut LoRa<impl RadioKind, impl DelayNs>) {
        let mut delay = self.delay;
        let mut lora = LoraPhy::new(lora, self.lora_config);
        let mut buffer = [0; MAX_VLP_PACKET_SIZE];
        let mut low_power_mode = false;

        let deserializer = |buffer: &[u8]| self.packet_builder.deserialize_uplink(buffer);

        loop {
            let result: Result<(), RadioError> = try {
                if !low_power_mode {
                    let tx_packet = self.tx_signal.wait().await;
                    let tx_packet_serialized = self
                        .packet_builder
                        .serialize_downlink(&mut buffer, &tx_packet);
                    lora.tx(tx_packet_serialized).await?;
                }

                let listen_mode = if low_power_mode {
                    RxMode::DutyCycle(DutyCycleParams {
                        rx_time: 10_000 / 15,     // 10ms
                        sleep_time: 100_000 / 15, // 100ms
                    })
                } else {
                    RxMode::Single(100)
                };
                match lora.rx(listen_mode, &mut buffer, deserializer).await {
                    Ok(Some((packet, packet_status))) => {
                        if let VLPUplinkPacket::LowPowerModePacket(LowPowerModePacket {
                            enabled,
                            ..
                        }) = &packet
                        {
                            low_power_mode = *enabled;
                        }
                        self.rx_signal.signal((packet, packet_status));

                        let ack_packet_serialized = self.packet_builder.serialize_downlink(
                            &mut buffer,
                            &AckPacket {
                                timestamp: self.unix_clock.now_ms(),
                            }
                            .into(),
                        );
                        lora.tx(ack_packet_serialized).await?;
                    }
                    Err(RadioError::ReceiveTimeout) | Ok(None) => {}
                    Err(e) => {
                        Err(e)?;
                    }
                }
            };
            if let Err(e) = result {
                log_error!("Error in VLP uplink client: {:?}", e);
                delay.delay_ms(1000).await;
            }
        }
    }
}
