use core::mem::size_of;

use super::delta_factory::{DeltaFactory, Deltable};
use either::Either;
use rkyv::ser::serializers::BufferSerializer;
use rkyv::ser::Serializer;
use rkyv::{Archive, Serialize};

pub struct DeltaLogger<T, W>
where
    W: vlfs::AsyncWriter,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<<T as Archive>::Archived>()]>>,
    T::DeltaType: Archive+ Serialize<BufferSerializer<[u8; size_of::<<T as Archive>::Archived>()]>>,
    [(); size_of::<<T as Archive>::Archived>()]:,
{
    factory: DeltaFactory<T>,
    writer: W,
    buffer: [u8; size_of::<<T as Archive>::Archived>()],
}

impl<T, W> DeltaLogger<T, W>
where
    W: vlfs::AsyncWriter,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<<T as Archive>::Archived>()]>>,
    T::DeltaType: Archive+ Serialize<BufferSerializer<[u8; size_of::<<T as Archive>::Archived>()]>>,
    [(); size_of::<<T as Archive>::Archived>()]:,
{
    pub fn new(writer: W) -> Self {
        Self {
            factory: DeltaFactory::new(),
            writer,
            buffer: [0; size_of::<<T as Archive>::Archived>()],
        }
    }

    pub async fn log(&mut self, value: T) -> Result<(), W::Error> {
        let mut serializer = BufferSerializer::new(self.buffer);
        match self.factory.push(value) {
            Either::Left(value) => {
                serializer.serialize_value(&value).unwrap();
                let buffer = serializer.into_inner();
                self.writer.extend_from_u8(0x69).await?;
                self.writer.extend_from_slice(&buffer).await?;
            }
            Either::Right(delta) =>  {
                serializer.serialize_value(&delta).unwrap();
                let buffer = serializer.into_inner();
                let buffer = &buffer[..size_of::<<T::DeltaType as Archive>::Archived>()];
                self.writer.extend_from_u8(0x42).await?;
                self.writer.extend_from_slice(buffer).await?;
            },
        };

        Ok(())
    }

    pub fn into_writer(self) -> W {
        self.writer
    }
}
