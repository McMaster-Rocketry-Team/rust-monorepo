use bitvec::order::Lsb0;

pub type SerializeBitOrder = Lsb0;

pub mod bitslice_io;
pub mod bitvec_serialize_traits;
pub mod delta_logger;
pub mod ring_delta_logger;
