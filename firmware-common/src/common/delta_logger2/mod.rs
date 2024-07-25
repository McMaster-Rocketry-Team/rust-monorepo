use std::marker::PhantomData;

use bitvec::prelude::*;
use either::Either;

use crate::driver::timestamp::TimestampType;

use super::{
    delta_factory::{DeltaFactory, Deltable},
    fixed_point::F64FixedPointFactory,
    sensor_reading::SensorReading,
    variable_int::VariableIntTrait,
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

impl BitSliceWritable for f64 {
    fn write<O: BitOrder>(self, slice: &mut BitSlice<u8, O>) -> usize {
        let data = self.to_be_bytes();
        let data: &BitSlice<u8, O> = data.view_bits();
        (&mut slice[..64]).copy_from_bitslice(data);
        64
    }
}

pub struct BitSliceWriter<O: BitOrder, const N: usize> {
    len: usize,
    data: BitArray<[u8; N], O>,
}

impl<O: BitOrder, const N: usize> BitSliceWriter<O, N> {
    pub fn write(&mut self, value: impl BitSliceWritable) {
        let len = value.write(&mut self.data[self.len..]);
        self.len += len;
    }

    /// returns a slice of the full bytes
    /// e.g. if the length is 20 bits, this will return the first 2 bytes
    pub fn view_full_byte_slice(&self) -> &[u8] {
        let full_bytes = self.len / 8;
        &self.data.as_raw_slice()[..full_bytes]
    }

    /// returns a slice of the data, including bytes that are not fully filled
    /// e.g. if the length is 20 bits, this will return the first 3 bytes
    pub fn view_all_data_slice(&self) -> &[u8] {
        let bytes = (self.len + 7) / 8;
        &self.data.as_raw_slice()[..bytes]
    }

    /// removes the full bytes from the slice
    /// e.g. if the length is 20 bits, this will remove the first 2 bytes
    /// and the new length will be 4 bits
    pub fn clear_full_byte_slice(&mut self) {
        let full_bytes = self.len / 8;
        self.len = self.len % 8;
        let raw_slice = self.data.as_raw_mut_slice();
        raw_slice[0] = raw_slice[full_bytes];
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<O: BitOrder, const N: usize> Default for BitSliceWriter<O, N> {
    fn default() -> Self {
        Self {
            len: 0,
            data: Default::default(),
        }
    }
}

pub trait BitArraySerializable {
    fn serialize<O: BitOrder, const N: usize>(&self, writer: &mut BitSliceWriter<O, N>);
}

#[derive(Debug, Clone)]
struct Timestamp<F: F64FixedPointFactory>(f64, PhantomData<F>);

impl<F: F64FixedPointFactory> From<f64> for Timestamp<F> {
    fn from(value: f64) -> Self {
        Self(value, PhantomData)
    }
}

impl<F: F64FixedPointFactory> Deltable for Timestamp<F> {
    type DeltaType = TimestampDelta<F>;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self(self.0 + F::to_float(delta.0.clone()), PhantomData))
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(TimestampDelta(F::to_fixed_point(self.0 - other.0)?))
    }
}

#[derive(Debug, Clone)]
struct TimestampDelta<F: F64FixedPointFactory>(<F::VI as VariableIntTrait>::Packed);

pub struct DeltaLogger<TM, T, W, F>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    F: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 8]:,
{
    factory: DeltaFactory<T::Data>,
    timestamp_factory: DeltaFactory<Timestamp<F>>,
    writer: W,
    bit_writer: BitSliceWriter<Msb0, { size_of::<T::Data>() + 8 }>,
}

impl<TM, T, W, F> DeltaLogger<TM, T, W, F>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    F: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 8]:,
{
    pub fn new(writer: W) -> Self {
        Self {
            factory: DeltaFactory::new(),
            timestamp_factory: DeltaFactory::new(),
            writer,
            bit_writer: Default::default(),
        }
    }

    // header:
    // 00 -> delta timestamp, delta data
    // 01 -> delta timestamp, full data
    // 10 -> end of byte
    // 11 -> full timestamp, full data
    pub async fn log(&mut self, reading: T) -> Result<(), W::Error> {
        match self.timestamp_factory.push(reading.get_timestamp().into()) {
            Either::Left(full_timestamp) => {
                self.bit_writer.write(true);
                self.bit_writer.write(true);
                self.bit_writer.write(full_timestamp.0);
                reading.into_data().serialize(&mut self.bit_writer);
            }
            Either::Right(delta_timestamp) => {
                self.bit_writer.write(false);
                match self.factory.push(reading.into_data()) {
                    Either::Left(data) => {
                        self.bit_writer.write(true);
                        self.bit_writer.write(delta_timestamp.0);
                        data.serialize(&mut self.bit_writer);
                    }
                    Either::Right(delta) => {
                        self.bit_writer.write(false);
                        self.bit_writer.write(delta_timestamp.0);
                        delta.serialize(&mut self.bit_writer);
                    }
                }
            }
        }

        self.writer
            .write_all(self.bit_writer.view_full_byte_slice())
            .await?;
        self.bit_writer.clear_full_byte_slice();

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), W::Error> {
        self.bit_writer.write(true);
        self.bit_writer.write(false);
        self.writer
            .write_all(self.bit_writer.view_all_data_slice())
            .await?;
        self.writer.flush().await?;
        self.bit_writer.clear();
        Ok(())
    }
}
