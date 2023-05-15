use super::*;
use bitflags::bitflags;
use heapless::Vec;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Flags: u8 {
        const ESTABLISH = 0x80;
        const ENCRYPTION = 0x40;
        const COMPRESSION = 0x20;
        const RELIABLE = 0x10;

        const HANDOFF = 0x04;
        const PSH = 0x02;
        const ACK = 0x01;
    }
}

impl Flags {
    pub fn establish_with_params(params: &SocketParams) -> Flags {
        let mut flags = Flags::ESTABLISH;
        if params.encryption {
            flags |= Flags::ENCRYPTION;
        }
        if params.compression {
            flags |= Flags::COMPRESSION;
        }
        if params.reliability {
            flags |= Flags::RELIABLE;
        }

        flags
    }
}

pub struct Packet {
    pub flags: Flags,
    pub seqnum: u16,
    pub payload: Option<Vec<u8, 222>>,
}

impl Packet {
    pub fn serialize(&self) -> Vec<u8, 222> {
        let mut buf = packet![self.flags.bits()];
        buf.push((self.seqnum >> 8) as u8);
        buf.push((self.seqnum & 0xff) as u8);
        if let Some(payload) = &self.payload {
            buf.extend_from_slice(&payload[..]);
        }

        buf
    }

    pub fn deserialize(mut packet: Vec<u8, 222>) -> Result<Packet, FramingError> {
        if packet.len() < 3 {
            return Err(FramingError::MalformedPacket(packet));
        }

        let mut _self = Packet {
            flags: Flags::empty(),
            seqnum: 0xFFFF,
            payload: None,
        };

        match Flags::from_bits(packet[0]) {
            Some(flags) => _self.flags = flags,
            None => return Err(FramingError::MalformedPacket(packet)),
        }

        _self.seqnum = ((packet[1] as u16) << 8) | (packet[2] as u16);

        if _self.flags.contains(Flags::PSH) {
            packet.remove(0);
            packet.remove(0);
            packet.remove(0);
            _self.payload = Some(packet);
        }

        Ok(_self)
    }
}

#[derive(Clone, Debug, PartialEq, defmt::Format)]
pub enum FramingError {
    SocketDisconnected,
    StateError(ConnectionState),
    MalformedPacket(#[defmt(Debug2Format)] Vec<u8, 222>),
}
