use crc::{Crc, CRC_8_SMBUS};
use cryptoxide::chacha20::ChaCha20;
use lora_modulation::BaseBandModulationParams;
use rkyv::{Archive, Deserialize, Serialize};

use crate::{common::unix_clock::UnixClock, create_serialized_enum, Clock};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct VerticalCalibrationPackage {
    timestamp: f64,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct SoftArmPackage {
    timestamp: f64,
    armed: bool,
}

create_serialized_enum!(
    VLPPacketWriter,
    VLPPacketReader,
    VLPPacket,
    (0, VerticalCalibrationPackage),
    (1, SoftArmPackage)
);

pub struct LoraPacketBuilder<'a, CL: Clock> {
    lora_config: BaseBandModulationParams,
    crc: Crc<u8>,
    key: [u8; 32],
    unix_clock: UnixClock<'a, CL>,
}

impl<'a, CL: Clock> LoraPacketBuilder<'a, CL> {
    pub fn new(
        unix_clock: UnixClock<'a, CL>,
        lora_config: BaseBandModulationParams,
        key: [u8; 32],
    ) -> Self {
        LoraPacketBuilder {
            lora_config,
            crc: Crc::<u8>::new(&CRC_8_SMBUS),
            key,
            unix_clock,
        }
    }

    fn get_nonce(&self, time: f64) -> [u8; 8] {
        let mut now = time as u64;
        // cloesest 100ms
        now = now - now % 100;
        now.to_le_bytes()
    }

    /// Should be called right before sending the packet
    pub fn serialize_uplink<'b>(&self, buffer: &'b mut [u8], packet: &VLPPacket) -> &'b [u8] {
        // serialize
        let len = packet.write_to_buffer(buffer);

        // crc
        buffer[len] = self.crc.checksum(&buffer[..len]);

        // encrypt
        let mut cipher = ChaCha20::new(&self.key, &self.get_nonce(self.unix_clock.now_ms()));
        cipher.process_mut(&mut buffer[..(len + 1)]);

        &buffer[..(len + 1)]
    }

    fn get_past_nonce(&self, packet_length: usize) -> ([u8; 8], [u8; 8]) {
        let now = self.unix_clock.now_ms() as u64;
        let air_time = self
            .lora_config
            .time_on_air_us(Some(4), true, packet_length as u8)
            / 1000;
        let sent_time = now - air_time as u64;

        let module = sent_time % 100;

        (
            (sent_time - module).to_le_bytes(),
            if module < 50 {
                sent_time - module - 100
            } else {
                sent_time - module + 100
            }
            .to_le_bytes(),
        )
    }

    pub fn deserialize_uplink(&self, buffer: &[u8]) -> Option<VLPPacket> {
        if buffer.len() <= 1 {
            log_info!("Received Lora message too short");
            return None;
        }

        // decrypt
        let mut decrypt_buffer = [0u8; size_of::<<VLPPacket as Archive>::Archived>() + 1];
        if buffer.len() > decrypt_buffer.len() {
            log_info!("Received Lora message too long");
            return None;
        }
        let mut decrypt_buffer = &mut decrypt_buffer[..buffer.len()];

        let mut check_nonce = |nonce: &[u8]| {
            let mut cipher = ChaCha20::new(&self.key, nonce);
            cipher.process(buffer, &mut decrypt_buffer);

            let calculated_crc = self
                .crc
                .checksum(&decrypt_buffer[..(decrypt_buffer.len() - 1)]);
            calculated_crc == decrypt_buffer[decrypt_buffer.len() - 1]
        };

        let (nonce_1, nonce_2) = self.get_past_nonce(buffer.len());

        if check_nonce(&nonce_1) || check_nonce(&nonce_2) {
            // deserialize
            VLPPacket::from_buffer(&decrypt_buffer[..(decrypt_buffer.len() - 1)])
        } else {
            log_info!("Received Lora message with invalid CRC");
            None
        }
    }
}
