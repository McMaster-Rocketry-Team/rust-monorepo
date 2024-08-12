use bitvec::order::Lsb0;

pub type SerializeBitOrder = Lsb0;

pub mod bitslice_primitive;
pub mod bitslice_serialize;
pub mod delta_factory;
pub mod delta_logger;
pub mod ring_delta_logger;
pub mod tiered_ring_delta_logger;
pub mod ring_file_writer;
pub mod buffered_logger;
pub mod delta_logger_trait;
pub mod merged_logger;

pub mod prelude {
    pub use super::bitslice_serialize::{BitArraySerializable, BitSliceReader, BitSliceWriter};
    pub use super::delta_factory::Deltable;
    pub use super::ring_file_writer::RingFileWriter;
    pub use super::delta_logger_trait::DeltaLoggerTrait;
    pub use super::bitslice_primitive::BitSlicePrimitive;
}
