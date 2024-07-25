use std::{marker::PhantomData, mem::transmute};

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

trait FromBitSlice<O: BitOrder>: Sized {
    fn from_bit_slice(slice: &BitSlice<u8, O>) -> Self;

    fn bit_size() -> usize;
}

impl<O: BitOrder> FromBitSlice<O> for bool {
    fn from_bit_slice(slice: &BitSlice<u8, O>) -> Self {
        slice[0]
    }

    fn bit_size() -> usize {
        1
    }
}

impl FromBitSlice<Msb0> for u8 {
    fn from_bit_slice(slice: &BitSlice<u8, Msb0>) -> Self {
        let slice = &slice[..8];
        slice.load_be::<u8>()
    }
    fn bit_size() -> usize {
        8
    }
}

impl FromBitSlice<Lsb0> for u8 {
    fn from_bit_slice(slice: &BitSlice<u8, Lsb0>) -> Self {
        let slice = &slice[..8];
        slice.load_be::<u8>()
    }
    fn bit_size() -> usize {
        8
    }
}

impl FromBitSlice<Lsb0> for f32 {
    fn from_bit_slice(slice: &BitSlice<u8, Lsb0>) -> Self {
        let slice = &slice[..32];
        unsafe { transmute(slice.load_be::<u32>()) }
    }
    fn bit_size() -> usize {
        32
    }
}

impl FromBitSlice<Msb0> for f32 {
    fn from_bit_slice(slice: &BitSlice<u8, Msb0>) -> Self {
        let slice = &slice[..32];
        unsafe { transmute(slice.load_be::<u32>()) }
    }
    fn bit_size() -> usize {
        32
    }
}

impl FromBitSlice<Lsb0> for f64 {
    fn from_bit_slice(slice: &BitSlice<u8, Lsb0>) -> Self {
        let slice = &slice[..64];
        unsafe { transmute(slice.load_be::<u64>()) }
    }

    fn bit_size() -> usize {
        64
    }
}

impl FromBitSlice<Msb0> for f64 {
    fn from_bit_slice(slice: &BitSlice<u8, Msb0>) -> Self {
        let slice = &slice[..64];
        unsafe { transmute(slice.load_be::<u64>()) }
    }

    fn bit_size() -> usize {
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

pub struct BitSliceReader<O: BitOrder, const N: usize> {
    buffer: BitArray<[u8; N], O>,
    start: usize,
    end: usize,
}

impl<O: BitOrder, const N: usize> Default for BitSliceReader<O, N> {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            start: 0,
            end: 0,
        }
    }
}

impl<O: BitOrder, const N: usize> BitSliceReader<O, N> {
    fn len_bits(&self) -> usize {
        self.end - self.start
    }

    fn free_bytes(&self) -> usize {
        (self.buffer.len() - self.len_bits()) / 8
    }

    /// move existing data to the beginning of the buffer and append new data
    fn replenish_bytes(&mut self, data: &[u8]) {
        let len_bits = self.len_bits();
        let dest = (8 - len_bits % 8) % 8;
        self.buffer.copy_within(self.start..self.end, dest);
        self.start = dest;
        self.end = dest + len_bits;

        let new_start_byte_i = self.end / 8;
        (&mut self.buffer.as_raw_mut_slice()[new_start_byte_i..(new_start_byte_i + data.len())])
            .copy_from_slice(data);
        self.end += data.len() * 8;
    }

    fn read<T: FromBitSlice<O>>(&mut self) -> Option<T> {
        if self.len_bits() < T::bit_size() {
            return None;
        }
        let data = T::from_bit_slice(&self.buffer[self.start..self.end]);
        self.start += T::bit_size();
        Some(data)
    }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bit_slice_reader() {
        let mut reader: BitSliceReader<Msb0, 4> = Default::default();
        assert_eq!(reader.len_bits(), 0);
        assert_eq!(reader.free_bytes(), 4);

        reader.replenish_bytes(&[0xFF, 0b01111111]);
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<bool>(), Some(true));
        assert_eq!(reader.len_bits(), 15);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<u8>(), Some(0b11111110));
        assert_eq!(reader.read::<u8>(), None);
        assert_eq!(reader.len_bits(), 7);
        assert_eq!(reader.free_bytes(), 3);

        reader.replenish_bytes(&[0xFF; 3]);
        assert_eq!(reader.len_bits(), 7 + 3 * 8);
        assert_eq!(reader.free_bytes(), 0);
        assert_eq!(reader.start, 1);

        for _ in 0..7 {
            reader.read::<bool>();
        }

        assert_eq!(reader.len_bits(), 3 * 8);
        assert_eq!(reader.free_bytes(), 1);
        reader.replenish_bytes(&[0xFF]);
        assert_eq!(reader.len_bits(), 4 * 8);
        assert_eq!(reader.free_bytes(), 0);
    }
}
