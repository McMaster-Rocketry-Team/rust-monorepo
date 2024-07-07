use packed_struct::prelude::*;

use super::message::CanBusMessage;


#[derive(PackedStruct, Clone, Copy, Debug, PartialEq, Eq)]
#[packed_struct(bit_numbering = "msb0", endian = "msb", size_bytes = "8")]
pub struct UnixTimeMessage {
    /// Current milliseconds since Unix epoch, floored to the nearest ms
    pub timestamp: u64,
}

impl CanBusMessage for UnixTimeMessage {
    fn message_type() -> u8 {
        0
    }
}

#[derive(PackedStruct, Clone, Copy, Debug, PartialEq, Eq)]
#[packed_struct(bit_numbering = "msb0", endian = "msb", size_bytes = "1")]
pub struct AvionicsStatusMessage {
    #[packed_field(bits = "0")]
    pub low_power: bool,
    #[packed_field(bits = "1")]
    pub armed: bool,
}

impl CanBusMessage for AvionicsStatusMessage {
    fn message_type() -> u8 {
        1
    }
}

#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightEvent {
    Ignition = 0,
    Coast = 1,
    Apogee = 2,
    Landed = 3,
}

#[derive(PackedStruct, Clone, Copy, Debug, PartialEq, Eq)]
#[packed_struct(bit_numbering = "msb0", endian = "msb", size_bytes = "9")]
pub struct FlightEventMessage {
    /// Current milliseconds since Unix epoch, floored to the nearest ms
    pub timestamp: u64,
    #[packed_field(bits = "64..=66", ty = "enum")]
    pub event: FlightEvent,
}

impl CanBusMessage for FlightEventMessage {
    fn message_type() -> u8 {
        2
    }
}

#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthState {
    Healthy = 0,
    Degraded = 1,
    UnHealthy = 2,
}

#[derive(PackedStruct, Clone, Copy, Debug, PartialEq, Eq)]
#[packed_struct(bit_numbering = "msb0", endian = "msb", size_bytes = "1")]
pub struct HealthMessage {
    #[packed_field(bits = "0..=1", ty = "enum")]
    pub state: HealthState,
}

impl CanBusMessage for HealthMessage {
    fn message_type() -> u8 {
        3
    }
}
