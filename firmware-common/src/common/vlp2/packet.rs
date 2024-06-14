use crc::{Crc, CRC_8_SMBUS};
use cryptoxide::chacha20::ChaCha20;
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive, Serialize,
};

use crate::{common::unix_clock::UnixClock, Clock};

pub struct LoraPacketBuilder<'a, CL: Clock> {
    crc: Crc<u8>,
    key: [u8; 32],
    unix_clock: UnixClock<'a, CL>,
}

impl<'a, CL: Clock> LoraPacketBuilder<'a, CL> {
    pub fn new(unix_clock: UnixClock<'a, CL>, key: [u8; 32]) -> Self {
        LoraPacketBuilder {
            crc: Crc::<u8>::new(&CRC_8_SMBUS),
            key,
            unix_clock,
        }
    }

    fn get_nonce(&self) -> [u8; 8] {
        let mut now = self.unix_clock.now_ms() as u64;
        // cloesest 100ms
        now = now - now % 100;
        now.to_le_bytes()
    }

    /// Should be called right before sending the packet
    pub fn serialize_uplink<'b, T>(&self, buffer: &'b mut [u8], packet: T) -> &'b [u8]
    where
        T: Archive + Serialize<BufferSerializer<&'b mut [u8]>>,
    {
        // serialize
        let mut serializer = BufferSerializer::new(buffer);
        serializer.serialize_value(&packet).unwrap();
        let buffer = serializer.into_inner();

        // crc
        buffer[size_of::<T::Archived>()] = self.crc.checksum(&buffer[..size_of::<T::Archived>()]);

        // encrypt
        let mut cipher = ChaCha20::new(&self.key, &self.get_nonce());
        cipher.process_mut(&mut buffer[..(size_of::<T::Archived>() + 1)]);

        &buffer[..(size_of::<T::Archived>() + 1)]
    }
}
