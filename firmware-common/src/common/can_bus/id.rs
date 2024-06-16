use bilge::prelude::*;

#[bitsize(29)]
#[derive(FromBits)]
pub struct CanBusExtendedId {
    priority: u3,
    message_type: u6,
    node_id: u8,
    message_id: u12,
}

pub const VL_NODE_ID: u8 = 0x69;