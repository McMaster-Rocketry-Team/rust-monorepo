use crc::{Crc, CRC_8_SMBUS};
use cryptoxide::chacha20::ChaCha20;
use heapless::Vec;
use lora_modulation::BaseBandModulationParams;

use crate::{
    common::{
        delta_logger::prelude::{BitArraySerializable, BitSliceReader, BitSliceWriter},
        unix_clock::UnixClock,
        vlp2::{packet2::*, telemetry_packet2::TelemetryPacket},
    },
    Clock,
};
use packed_struct::prelude::*;

use super::packet2::{VLPDownlinkPacket, VLPUplinkPacket};

pub const MAX_VLP_PACKAGE_SIZE: usize = 48;

pub struct VLPPacketBuilder<'a, 'b, CL: Clock> {
    lora_config: BaseBandModulationParams,
    crc: Crc<u8>,
    uplink_key: &'b [u8; 32],
    downlink_key: [u8; 32],
    unix_clock: UnixClock<'a, CL>,
    bit_slice_writer: BitSliceWriter<MAX_VLP_PACKAGE_SIZE>,
    bit_slice_reader: BitSliceReader<MAX_VLP_PACKAGE_SIZE>,
}

impl<'a, 'b, CL: Clock> VLPPacketBuilder<'a, 'b, CL> {
    pub fn new(
        unix_clock: UnixClock<'a, CL>,
        lora_config: BaseBandModulationParams,
        key: &'b [u8; 32],
    ) -> Self {
        let mut downlink_key = [0u8; 32];
        let mut chacha = ChaCha20::new(key, &[0; 8]);
        chacha.process_mut(&mut downlink_key);
        VLPPacketBuilder {
            lora_config,
            crc: Crc::<u8>::new(&CRC_8_SMBUS),
            uplink_key: key,
            downlink_key,
            unix_clock,
            bit_slice_writer: Default::default(),
            bit_slice_reader: Default::default(),
        }
    }

    fn get_nonce(time: f64) -> [u8; 8] {
        let mut now = time as u64;
        // cloesest 100ms
        now = now - now % 100;
        now.to_le_bytes()
    }

    fn get_past_nonce(&self, packet_length: usize) -> Vec<(i32, [u8; 8]), 10> {
        let now = self.unix_clock.now_ms() as u64;
        let air_time = self
            .lora_config
            .time_on_air_us(Some(4), true, packet_length as u8)
            / 1000;
        let sent_time = now.checked_sub(air_time as u64).unwrap_or(0);

        let module = sent_time % 100;

        let mut result = Vec::new();

        let start = sent_time
            .checked_sub(module)
            .unwrap_or(0)
            .checked_sub(500)
            .unwrap_or(0);
        for i in 0..10 {
            result
                .push((
                    (i as i32 * 100 - 500) as i32,
                    (start + i * 100).to_le_bytes(),
                ))
                .unwrap();
        }

        result
    }

    pub fn serialize_uplink(
        &mut self,
        buffer: &mut Vec<u8, MAX_VLP_PACKAGE_SIZE>,
        packet: &VLPUplinkPacket,
    ) -> Result<(), ()> {
        buffer.clear();

        // serialize
        self.bit_slice_writer.clear();
        let packet_type: u8 = match packet {
            VLPUplinkPacket::VerticalCalibrationPacket(_) => 0,
            VLPUplinkPacket::SoftArmPacket(_) => 1,
            VLPUplinkPacket::LowPowerModePacket(_) => 2,
            VLPUplinkPacket::ResetPacket(_) => 3,
            VLPUplinkPacket::DeleteLogsPacket(_) => 4,
        };
        let packet_type: Integer<u8, packed_bits::Bits<3>> = packet_type.into();

        self.bit_slice_writer.write(packet_type);
        match packet {
            VLPUplinkPacket::VerticalCalibrationPacket(packet) => {
                packet.serialize(&mut self.bit_slice_writer)
            }
            VLPUplinkPacket::SoftArmPacket(packet) => packet.serialize(&mut self.bit_slice_writer),
            VLPUplinkPacket::LowPowerModePacket(packet) => {
                packet.serialize(&mut self.bit_slice_writer)
            }
            VLPUplinkPacket::ResetPacket(packet) => packet.serialize(&mut self.bit_slice_writer),
            VLPUplinkPacket::DeleteLogsPacket(packet) => {
                packet.serialize(&mut self.bit_slice_writer)
            }
        };

        let data = self.bit_slice_writer.view_all_data_slice();
        buffer.extend_from_slice(data)?;

        // crc
        buffer
            .push(self.crc.checksum(buffer.as_slice()))
            .map_err(|_| ())?;

        // ecc
        let ecc_len = calculate_ecc_length_from_data_length(buffer.len());
        let enc = reed_solomon::Encoder::new(ecc_len);
        let encoded = enc.encode(buffer.as_slice());
        buffer.extend_from_slice(encoded.ecc())?;

        // encrypt
        let mut cipher = ChaCha20::new(self.uplink_key, &Self::get_nonce(self.unix_clock.now_ms()));
        cipher.process_mut(buffer.as_mut_slice());

        Ok(())
    }

    pub fn deserialize_uplink(
        &mut self,
        buffer: &Vec<u8, MAX_VLP_PACKAGE_SIZE>,
    ) -> Result<VLPUplinkPacket, ()> {
        if buffer.len() <= 8 {
            log_info!("Received Lora message too short");
            return Err(());
        }

        let ecc_len = calculate_ecc_length_from_total_length(buffer.len());
        let dec = reed_solomon::Decoder::new(ecc_len);

        for (offset, nonce) in self.get_past_nonce(buffer.len()) {
            let mut buffer = buffer.clone();

            // decrypt
            let mut cipher = ChaCha20::new(self.uplink_key, &nonce);
            cipher.process_mut(buffer.as_mut_slice());

            // ecc
            if let Ok(recovered) = dec.correct(buffer.as_slice(), None) {
                buffer.clear();
                buffer.extend_from_slice(recovered.data())?;
            } else {
                continue;
            };

            // crc
            let calculated_crc = self.crc.checksum(&buffer.as_slice()[..buffer.len() - 1]);
            if calculated_crc == buffer[buffer.len() - 1] {
                buffer.pop();
                self.bit_slice_reader.clear();
                self.bit_slice_reader.replenish_bytes(buffer.as_slice());
                let packet_type: Integer<u8, packed_bits::Bits<3>> =
                    self.bit_slice_reader.read().unwrap();
                let packet_type: u8 = packet_type.into();
                let packet = match packet_type {
                    0 => VLPUplinkPacket::VerticalCalibrationPacket(
                        VerticalCalibrationPacket::deserialize(&mut self.bit_slice_reader),
                    ),
                    1 => VLPUplinkPacket::SoftArmPacket(SoftArmPacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    2 => VLPUplinkPacket::LowPowerModePacket(LowPowerModePacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    3 => VLPUplinkPacket::ResetPacket(ResetPacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    4 => VLPUplinkPacket::DeleteLogsPacket(DeleteLogsPacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    _ => {
                        continue;
                    }
                };
                log_info!("Received Lora message with offset {}ms", offset);
                return Ok(packet);
            } else {
                continue;
            }
        }

        Err(())
    }

    pub fn serialize_downlink(
        &mut self,
        buffer: &mut Vec<u8, MAX_VLP_PACKAGE_SIZE>,
        packet: &VLPDownlinkPacket,
    ) -> Result<(), ()> {
        buffer.clear();

        // serialize
        self.bit_slice_writer.clear();
        let packet_type: u8 = match packet {
            VLPDownlinkPacket::AckPacket(_) => 0,
            VLPDownlinkPacket::TelemetryPacket(_) => 1,
        };
        let packet_type: Integer<u8, packed_bits::Bits<1>> = packet_type.into();

        self.bit_slice_writer.write(packet_type);
        match packet {
            VLPDownlinkPacket::AckPacket(packet) => packet.serialize(&mut self.bit_slice_writer),
            VLPDownlinkPacket::TelemetryPacket(packet) => {
                packet.serialize(&mut self.bit_slice_writer)
            }
        }

        let data = self.bit_slice_writer.view_all_data_slice();
        buffer.extend_from_slice(data)?;

        // crc
        let mut digest = self.crc.digest();
        digest.update(buffer.as_slice());
        digest.update(&self.downlink_key);
        digest.update(&Self::get_nonce(self.unix_clock.now_ms()));
        buffer.push(digest.finalize()).map_err(|_| ())?;

        // ecc
        let ecc_len = calculate_ecc_length_from_data_length(buffer.len());
        let enc = reed_solomon::Encoder::new(ecc_len);
        let encoded = enc.encode(buffer.as_slice());
        buffer.extend_from_slice(encoded.ecc())?;

        Ok(())
    }

    pub fn deserialize_downlink(
        &mut self,
        buffer: &Vec<u8, MAX_VLP_PACKAGE_SIZE>,
    ) -> Result<VLPDownlinkPacket, ()> {
        if buffer.len() <= 8 {
            log_info!("Received Lora message too short");
            return Err(());
        }

        let ecc_len = calculate_ecc_length_from_total_length(buffer.len());
        let dec = reed_solomon::Decoder::new(ecc_len);

        for (offset, nonce) in self.get_past_nonce(buffer.len()) {
            let mut buffer = buffer.clone();

            // ecc
            if let Ok(recovered) = dec.correct(buffer.as_slice(), None) {
                buffer.clear();
                buffer.extend_from_slice(recovered.data())?;
            } else {
                continue;
            };

            // crc
            let mut digest = self.crc.digest();
            digest.update(&buffer.as_slice()[..buffer.len() - 1]);
            digest.update(&self.downlink_key);
            digest.update(&nonce);
            let calculated_crc = digest.finalize();
            if calculated_crc == buffer[buffer.len() - 1] {
                buffer.pop();
                self.bit_slice_reader.clear();
                self.bit_slice_reader.replenish_bytes(buffer.as_slice());
                let packet_type: Integer<u8, packed_bits::Bits<1>> =
                    self.bit_slice_reader.read().unwrap();
                let packet_type: u8 = packet_type.into();
                let packet = match packet_type {
                    0 => VLPDownlinkPacket::AckPacket(AckPacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    1 => VLPDownlinkPacket::TelemetryPacket(TelemetryPacket::deserialize(
                        &mut self.bit_slice_reader,
                    )),
                    _ => {
                        continue;
                    }
                };
                log_info!("Received Lora message with offset {}ms", offset);
                return Ok(packet);
            } else {
                continue;
            }
        }

        Err(())
    }
}

fn calculate_ecc_length_from_data_length(data_length: usize) -> usize {
    data_length / 4
}

fn calculate_ecc_length_from_total_length(total_length: usize) -> usize {
    total_length / 5
}

#[cfg(test)]
mod test {
    use crate::common::{
        unix_clock::UnixClockTask, vlp2::telemetry_packet2::FlightCoreStateTelemetry,
    };

    use super::*;

    #[test]
    fn test_ecc_length_calculation() {
        for len in 1usize..=48usize {
            let ecc_length = calculate_ecc_length_from_data_length(len);
            let total_length = len + ecc_length;
            let ecc_length2 = calculate_ecc_length_from_total_length(total_length);
            assert_eq!(ecc_length, ecc_length2);
        }
    }

    #[derive(Debug, Clone)]
    struct MockClock {}

    impl Clock for MockClock {
        fn now_ms(&self) -> f64 {
            1000.0
        }
    }

    #[test]
    fn test_serialize_deserialize_uplink() {
        let lora_config = BaseBandModulationParams::new(
            lora_modulation::SpreadingFactor::_10,
            lora_modulation::Bandwidth::_250KHz,
            lora_modulation::CodingRate::_4_5,
        );

        let clock = MockClock {};
        let unix_clock_task = UnixClockTask::new(clock);
        let unix_clock = unix_clock_task.get_clock();

        let key = [0x69u8; 32];

        let mut packet_builder = VLPPacketBuilder::new(unix_clock, lora_config, &key);

        let mut buffer = Vec::<u8, MAX_VLP_PACKAGE_SIZE>::new();
        let packet = VLPUplinkPacket::SoftArmPacket(SoftArmPacket {
            timestamp: 12345.67,
            armed: true,
        });
        packet_builder
            .serialize_uplink(&mut buffer, &packet)
            .unwrap();

        println!("serialized package len: {} {:02X?}", buffer.len(), buffer);

        let deserialized_packet = packet_builder.deserialize_uplink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[4] = 0xFF;
        let deserialized_packet = packet_builder.deserialize_uplink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[6] = 0xFF;
        packet_builder.deserialize_uplink(&buffer).unwrap_err();
    }

    #[test]
    fn test_serialize_deserialize_downlink() {
        let lora_config = BaseBandModulationParams::new(
            lora_modulation::SpreadingFactor::_10,
            lora_modulation::Bandwidth::_250KHz,
            lora_modulation::CodingRate::_4_5,
        );

        let clock = MockClock {};
        let unix_clock_task = UnixClockTask::new(clock);
        let unix_clock = unix_clock_task.get_clock();

        let key = [0x69u8; 32];

        let mut packet_builder = VLPPacketBuilder::new(unix_clock, lora_config, &key);

        let mut buffer = Vec::<u8, MAX_VLP_PACKAGE_SIZE>::new();
        let packet = VLPDownlinkPacket::TelemetryPacket(TelemetryPacket::new(
            false,
            342354.4,
            10,
            Some((0.234234, 34.234234)),
            7.6,
            25.5,
            true,
            true,
            340005,
            true,
            true,
            1234.5,
            3456.3,
            -200.1,
            350.3,
            FlightCoreStateTelemetry::Armed,
        ));
        packet_builder
            .serialize_downlink(&mut buffer, &packet)
            .unwrap();

        println!("serialized package len: {} {:02X?}", buffer.len(), buffer);

        let deserialized_packet = packet_builder.deserialize_downlink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[4] = 0xFF;
        let deserialized_packet = packet_builder.deserialize_downlink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[6] = 0xFF;
        let deserialized_packet = packet_builder.deserialize_downlink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[39] = 0xFF;
        let deserialized_packet = packet_builder.deserialize_downlink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[20] = 0xFF;
        let deserialized_packet = packet_builder.deserialize_downlink(&buffer).unwrap();
        assert_eq!(packet, deserialized_packet);

        buffer[30] = 0xFF;
        packet_builder.deserialize_downlink(&buffer).unwrap_err();
    }
}
