use crc::{Crc, CRC_8_SMBUS};
use cryptoxide::chacha20::ChaCha20;
use lora_modulation::BaseBandModulationParams;

use crate::{common::unix_clock::UnixClock, Clock};

use super::packet::{VLPDownlinkPacket, VLPUplinkPacket, MAX_VLP_UPLINK_PACKET_SIZE};

pub struct VLPPacketBuilder<'a, CL: Clock> {
    lora_config: BaseBandModulationParams,
    crc: Crc<u8>,
    uplink_key: [u8; 32],
    downlink_key: [u8; 32],
    unix_clock: UnixClock<'a, CL>,
}

impl<'a, CL: Clock> VLPPacketBuilder<'a, CL> {
    pub fn new(
        unix_clock: UnixClock<'a, CL>,
        lora_config: BaseBandModulationParams,
        key: [u8; 32],
    ) -> Self {
        let mut downlink_key = [0u8; 32];
        let mut chacha = ChaCha20::new(&key, &[0; 8]);
        chacha.process_mut(&mut downlink_key);
        VLPPacketBuilder {
            lora_config,
            crc: Crc::<u8>::new(&CRC_8_SMBUS),
            uplink_key: key,
            downlink_key,
            unix_clock,
        }
    }

    fn get_nonce(&self, time: f64) -> [u8; 8] {
        let mut now = time as u64;
        // cloesest 100ms
        now = now - now % 100;
        now.to_le_bytes()
    }

    pub fn serialize_uplink<'b>(&self, buffer: &'b mut [u8], packet: &VLPUplinkPacket) -> &'b [u8] {
        // serialize
        let len = packet.write_to_buffer(buffer);

        // crc
        buffer[len] = self.crc.checksum(&buffer[..len]);

        // encrypt
        let mut cipher = ChaCha20::new(&self.uplink_key, &self.get_nonce(self.unix_clock.now_ms()));
        cipher.process_mut(&mut buffer[..(len + 1)]);

        &buffer[..(len + 1)]
    }

    pub fn serialize_downlink<'b>(
        &self,
        buffer: &'b mut [u8],
        packet: &VLPDownlinkPacket,
    ) -> &'b [u8] {
        // serialize
        let len = packet.write_to_buffer(buffer);

        // crc
        let mut digest = self.crc.digest();
        digest.update(&buffer[..len]);
        digest.update(&self.downlink_key);
        digest.update(&self.get_nonce(self.unix_clock.now_ms()));
        buffer[len] = digest.finalize();

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

    pub fn deserialize_uplink(&self, buffer: &[u8]) -> Option<VLPUplinkPacket> {
        if buffer.len() <= 1 {
            log_info!("Received Lora message too short");
            return None;
        }

        // decrypt
        let mut deserialize_buffer = [0u8; MAX_VLP_UPLINK_PACKET_SIZE];
        if buffer.len() > deserialize_buffer.len() {
            log_info!("Received Lora message too long");
            return None;
        }
        let mut deserialize_buffer = &mut deserialize_buffer[..buffer.len()];

        let mut check_nonce = |nonce: &[u8]| {
            let mut cipher = ChaCha20::new(&self.uplink_key, nonce);
            cipher.process(buffer, &mut deserialize_buffer);

            let calculated_crc = self
                .crc
                .checksum(&deserialize_buffer[..(deserialize_buffer.len() - 1)]);
            calculated_crc == deserialize_buffer[deserialize_buffer.len() - 1]
        };

        let (nonce_1, nonce_2) = self.get_past_nonce(buffer.len());

        if check_nonce(&nonce_1) || check_nonce(&nonce_2) {
            // deserialize
            VLPUplinkPacket::from_buffer(&deserialize_buffer[..(deserialize_buffer.len() - 1)])
        } else {
            log_info!("Received Lora message with invalid CRC");
            None
        }
    }

    pub fn deserialize_downlink(&self, buffer: &[u8]) -> Option<VLPDownlinkPacket> {
        if buffer.len() <= 1 {
            log_info!("Received Lora message too short");
            return None;
        }

        // verify crc
        let check_nonce = |nonce: &[u8]| {
            let mut digest = self.crc.digest();
            digest.update(&buffer[..(buffer.len() - 1)]);
            digest.update(&self.downlink_key);
            digest.update(&nonce);
            let calculated_crc = digest.finalize();
            calculated_crc == buffer[buffer.len() - 1]
        };

        let (nonce_1, nonce_2) = self.get_past_nonce(buffer.len());

        if check_nonce(&nonce_1) || check_nonce(&nonce_2) {
            // deserialize
            VLPDownlinkPacket::from_buffer(&buffer[..(buffer.len() - 1)])
        } else {
            log_info!("Received Lora message with invalid CRC");
            None
        }
    }
}
