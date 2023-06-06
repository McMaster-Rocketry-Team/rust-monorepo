use rkyv::{ser::serializers::BufferSerializer, Serialize};

pub mod ack;
pub mod device;
pub mod event;
pub mod pyro;
pub mod continuity;
pub mod hardware_arming;

pub trait Package: Serialize<BufferSerializer<[u8; 128]>> {
    fn get_id() -> u8;
}
