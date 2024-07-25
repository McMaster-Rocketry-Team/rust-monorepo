use bitvec::prelude::*;
use either::Either;

use crate::driver::timestamp::TimestampType;

use super::{
    delta_factory::DeltaFactory,
    sensor_reading::SensorReading,
};

pub trait BitSliceWritable {
    fn write<O: BitOrder>(self, slice: &mut BitSlice<u8, O>) -> usize;
}

impl BitSliceWritable for bool {
    fn write<O: BitOrder>(self, slice: &mut BitSlice<u8, O>) -> usize {
        slice.set(0, self);
        1
    }
}

impl BitSliceWritable for u8 {
    fn write<O: BitOrder>(self, slice: &mut BitSlice<u8, O>) -> usize {
        let data = [self];
        let data: &BitSlice<u8, O> = data.view_bits();
        (&mut slice[..8]).copy_from_bitslice(data);
        8
    }
}

impl BitSliceWritable for f32 {
    fn write<O: BitOrder>(self, slice: &mut BitSlice<u8, O>) -> usize {
        let data = self.to_be_bytes();
        let data: &BitSlice<u8, O> = data.view_bits();
        (&mut slice[..32]).copy_from_bitslice(data);
        32
    }
}

pub struct BitSliceWriter<O: BitOrder,const N:usize>
{
    len: usize,
    data: BitArray<[u8; N], O>,
}

impl<O: BitOrder,const N:usize> BitSliceWriter<O, N> {
    pub fn write(&mut self, value: impl BitSliceWritable) {
        let len = value.write(&mut self.data[self.len..]);
        self.len += len;
    }

    pub fn view_full_byte_slice(&self) -> &[u8] {
        let full_bytes = self.len / 8;
        &self.data.as_raw_slice()[..full_bytes]
    }

    pub fn clear_full_byte_slice(&mut self) {
        let full_bytes = self.len / 8;
        self.len = self.len % 8;
        let raw_slice = self.data.as_raw_mut_slice();
        raw_slice[0] = raw_slice[full_bytes];
    }
}

impl<O: BitOrder,const N:usize> Default for BitSliceWriter<O, N> {
    fn default() -> Self {
        Self { len: 0, data: Default::default() }
    }
}

pub trait BitArraySerializable {
    fn serialize<O: BitOrder,const N:usize>(&self, writer: &mut BitSliceWriter<O,N>);
}

pub struct DeltaLogger<TM, T, W>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    [(); size_of::<T::Data>() + 8]:,
{
    factory: DeltaFactory<T::Data>,
    writer: W,
    bit_writer: BitSliceWriter<Msb0, {size_of::<T::Data>() + 8}>,
}

impl<TM, T, W> DeltaLogger<TM, T, W>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    [(); size_of::<T::Data>() + 8]:,
{
    pub fn new(writer: W) -> Self {
        Self {
            factory: DeltaFactory::new(),
            writer,
            bit_writer: Default::default(),
        }
    }

    pub async fn log(&mut self, reading: T) -> Result<(), W::Error> {
        match self.factory.push(reading.into_data()) {
            Either::Left(data) => {
                self.bit_writer.write(false);
                self.bit_writer.write(true);
                data.serialize(&mut self.bit_writer);
            }
            Either::Right(delta) => {
                self.bit_writer.write(false);
                self.bit_writer.write(true);
                delta.serialize(&mut self.bit_writer);
            }
        }

        self.writer
            .write_all(self.bit_writer.view_full_byte_slice())
            .await?;
        self.bit_writer.clear_full_byte_slice();

        Ok(())
    }
}
