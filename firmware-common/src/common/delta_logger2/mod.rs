use core::{future::Future, marker::PhantomData, mem::transmute};

use bitvec::prelude::*;
use either::Either;

use crate::driver::timestamp::TimestampType;

use super::{
    delta_factory::{DeltaFactory, Deltable, UnDeltaFactory},
    fixed_point::F64FixedPointFactory,
    sensor_reading::SensorReading,
    variable_int::VariableIntTrait,
};

pub type SerializeBitOrder = Lsb0;

pub trait BitSliceWritable {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize;
}

impl BitSliceWritable for bool {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        slice.set(0, self);
        1
    }
}

impl BitSliceWritable for u8 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = [self];
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..8]).copy_from_bitslice(data);
        8
    }
}

impl BitSliceWritable for f32 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..32]).copy_from_bitslice(data);
        32
    }
}

impl BitSliceWritable for f64 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..64]).copy_from_bitslice(data);
        64
    }
}

pub trait FromBitSlice: Sized {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self;

    fn len_bits() -> usize;
}

impl FromBitSlice for bool {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        slice[0]
    }

    fn len_bits() -> usize {
        1
    }
}

impl FromBitSlice for u8 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..8];
        slice.load_le::<u8>()
    }
    fn len_bits() -> usize {
        8
    }
}

impl FromBitSlice for f32 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..32];
        unsafe { transmute(slice.load_le::<u32>()) }
    }
    fn len_bits() -> usize {
        32
    }
}

impl FromBitSlice for f64 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..64];
        unsafe { transmute(slice.load_le::<u64>()) }
    }

    fn len_bits() -> usize {
        64
    }
}

pub struct BitSliceWriter<const N: usize> {
    len: usize,
    data: BitArray<[u8; N], SerializeBitOrder>,
}

impl<const N: usize> BitSliceWriter<N> {
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

impl<const N: usize> Default for BitSliceWriter<N> {
    fn default() -> Self {
        Self {
            len: 0,
            data: Default::default(),
        }
    }
}

pub trait BitArraySerializable {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>);
}

pub struct BitSliceReader<const N: usize> {
    buffer: BitArray<[u8; N], SerializeBitOrder>,
    start: usize,
    end: usize,
}

impl<const N: usize> Default for BitSliceReader<N> {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            start: 0,
            end: 0,
        }
    }
}

impl<const N: usize> BitSliceReader<N> {
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

    async fn replenish_bytes_async_mut<'a, 'b, FN, F, E>(&'a mut self, f: FN) -> Result<usize, E>
    where
        FN: FnOnce(&'b mut [u8]) -> F,
        F: Future<Output = Result<usize, E>>,
        'a: 'b,
    {
        let len_bits = self.len_bits();
        if len_bits == 0 {
            self.start = 0;
            self.end = 0;
        } else {
            let dest = (8 - len_bits % 8) % 8;
            self.buffer.copy_within(self.start..self.end, dest);
            self.start = dest;
            self.end = dest + len_bits;
        }

        let new_start_byte_i = self.end / 8;
        let buffer = &mut self.buffer.as_raw_mut_slice()[new_start_byte_i..];
        let read_len_bytes = f(buffer).await?;
        self.end += read_len_bytes * 8;
        Ok(read_len_bytes)
    }

    pub fn read<T: FromBitSlice>(&mut self) -> Option<T> {
        if self.len_bits() < T::len_bits() {
            return None;
        }
        let data = T::from_bit_slice(&self.buffer[self.start..self.end]);
        self.start += T::len_bits();
        Some(data)
    }

    pub fn skip_byte(&mut self) {
        self.start = (self.start + 7) / 8 * 8;
    }
}

pub trait BitArrayDeserializable {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self;

    fn len_bits() -> usize;
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
    [(); size_of::<T::Data>() + 10]:,
{
    factory: DeltaFactory<T::Data>,
    timestamp_factory: DeltaFactory<Timestamp<F>>,
    writer: W,
    bit_writer: BitSliceWriter<{ size_of::<T::Data>() + 10 }>,
}

impl<TM, T, W, F> DeltaLogger<TM, T, W, F>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    F: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
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

    pub fn into_writer(self) -> W {
        self.writer
    }
}

pub struct DeltaLoggerReader<TM, T, R, F>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    R: embedded_io_async::Read,
    F: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    factory: UnDeltaFactory<T::Data>,
    timestamp_factory: UnDeltaFactory<Timestamp<F>>,
    reader: R,
    bit_reader: BitSliceReader<{ size_of::<T::Data>() + 10 }>,
}

enum DeltaLoggerReaderResult<TM, T>
where
    TM: TimestampType,
    T: SensorReading<TM>,
{
    EOF,
    Data(T::NewType<TM>),
    TryAgain,
}

impl<TM, T, R, F> DeltaLoggerReader<TM, T, R, F>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    R: embedded_io_async::Read,
    F: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    pub fn new(reader: R) -> Self {
        Self {
            factory: UnDeltaFactory::new(),
            timestamp_factory: UnDeltaFactory::new(),
            reader,
            bit_reader: Default::default(),
        }
    }

    pub async fn read(&mut self) -> Result<Option<T::NewType<TM>>, R::Error> {
        loop {
            match self.inner_read().await? {
                DeltaLoggerReaderResult::EOF => {
                    return Ok(None);
                }
                DeltaLoggerReaderResult::Data(data) => {
                    return Ok(Some(data));
                }
                DeltaLoggerReaderResult::TryAgain => {
                    // noop
                }
            }
        }
    }

    async fn inner_read(&mut self) -> Result<DeltaLoggerReaderResult<TM, T>, R::Error> {
        if !self.ensure_at_least_bits(2).await? {
            return Ok(DeltaLoggerReaderResult::EOF);
        }

        let is_full_timestamp: bool = self.bit_reader.read().unwrap();
        let is_full_data: bool = self.bit_reader.read().unwrap();

        if is_full_timestamp && !is_full_data {
            // end of byte
            self.bit_reader.skip_byte();
            return Ok(DeltaLoggerReaderResult::TryAgain);
        }

        let timestamp = if is_full_timestamp {
            if !self.ensure_at_least_bits(64).await? {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            let timestamp: f64 = self.bit_reader.read().unwrap();
            Some(
                self.timestamp_factory
                    .push(Timestamp(timestamp, PhantomData)),
            )
        } else {
            if !self
                .ensure_at_least_bits(
                    <<F as F64FixedPointFactory>::VI as VariableIntTrait>::Packed::len_bits(),
                )
                .await?
            {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            let timestamp_delta_packed: <<F as F64FixedPointFactory>::VI as VariableIntTrait>::Packed  = self.bit_reader.read().unwrap();
            self.timestamp_factory
                .push_delta(TimestampDelta(timestamp_delta_packed))
        };

        let data = if is_full_data {
            if !self.ensure_at_least_bits(T::Data::len_bits()).await? {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            let data = self
                .factory
                .push(T::Data::deserialize(&mut self.bit_reader));
            Some(data)
        } else {
            if !self
                .ensure_at_least_bits(<T::Data as Deltable>::DeltaType::len_bits())
                .await?
            {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            self.factory
                .push_delta(<T::Data as Deltable>::DeltaType::deserialize(
                    &mut self.bit_reader,
                ))
        };

        if timestamp.is_none() || data.is_none() {
            return Ok(DeltaLoggerReaderResult::TryAgain);
        }

        let timestamp = timestamp.unwrap();
        let data = data.unwrap();

        Ok(DeltaLoggerReaderResult::Data(T::new(timestamp.0, data)))
    }

    async fn ensure_at_least_bits(&mut self, min_bits: usize) -> Result<bool, R::Error> {
        while self.bit_reader.len_bits() < min_bits {
            let read_bytes = self
                .bit_reader
                .replenish_bytes_async_mut(async |buffer| self.reader.read(buffer).await)
                .await?;
            if read_bytes == 0 {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use core::convert::Infallible;

    use crate::{
        common::test_utils::BufferWriter,
        driver::{
            adc::{ADCData, ADCReading},
            timestamp::BootTimestamp,
        },
        fixed_point_factory2, Volt,
    };

    use super::*;

    #[test]
    fn test_bit_slice_reader() {
        let mut reader: BitSliceReader<4> = Default::default();
        assert_eq!(reader.len_bits(), 0);
        assert_eq!(reader.free_bytes(), 4);

        reader.replenish_bytes(&[0xFF, 0b11111110]);
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<bool>(), Some(true));
        assert_eq!(reader.len_bits(), 15);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<u8>(), Some(0b01111111));
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

    #[tokio::test]
    async fn test_bit_slice_reader_async_replenish() {
        let mut reader: BitSliceReader<4> = Default::default();
        assert_eq!(reader.len_bits(), 0);
        assert_eq!(reader.free_bytes(), 4);

        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                assert_eq!(buffer.len(), 4);
                buffer[0] = 0xFF;
                buffer[1] = 0b11111110;
                Ok(2)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<bool>(), Some(true));
        assert_eq!(reader.len_bits(), 15);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<u8>(), Some(0b01111111));
        assert_eq!(reader.read::<u8>(), None);
        assert_eq!(reader.len_bits(), 7);
        assert_eq!(reader.free_bytes(), 3);

        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                buffer[0] = 0xFF;
                buffer[1] = 0xFF;
                buffer[2] = 0xFF;
                Ok(3)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 7 + 3 * 8);
        assert_eq!(reader.free_bytes(), 0);
        assert_eq!(reader.start, 1);

        for _ in 0..7 {
            reader.read::<bool>();
        }

        assert_eq!(reader.len_bits(), 3 * 8);
        assert_eq!(reader.free_bytes(), 1);
        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                buffer[0] = 0xFF;
                Ok(1)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 4 * 8);
        assert_eq!(reader.free_bytes(), 0);
    }

    #[test]
    fn test_bit_slice_reader_skip_byte() {
        let mut reader: BitSliceReader<4> = Default::default();
        reader.replenish_bytes(&[0xFF, 0b11111110]);

        reader.skip_byte();
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.read::<bool>(), Some(true));

        reader.skip_byte();
        assert_eq!(reader.len_bits(), 8);
        assert_eq!(reader.read::<u8>(), Some(0b11111110));
    }

    #[tokio::test]
    async fn test_delta_logger_write_read() {
        fixed_point_factory2!(TimestampFac, f64, 0.0, 510.0, 0.5);
        let readings = vec![
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(0.0, ADCData::new(0.0)),
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(501.0, ADCData::new(0.05)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(1000.0, ADCData::new(-0.05)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(1501.0, ADCData::new(-0.02)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(2000.0, ADCData::new(1.0)), // outside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(2501.0, ADCData::new(1.07)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5000.0, ADCData::new(1.1)), // timestamp outside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
            ADCReading::<Volt, BootTimestamp>::new::<BootTimestamp>(5501.0, ADCData::new(0.92)), // inside delta range
        ];

        let mut buffer = [0u8; 512];
        let writer = BufferWriter::new(&mut buffer);
        let mut logger =
            DeltaLogger::<BootTimestamp, ADCReading<Volt, BootTimestamp>, _, TimestampFac>::new(
                writer,
            );
        for reading in readings.iter() {
            logger.log(reading.clone()).await.unwrap();
        }
        logger.flush().await.unwrap();

        let reader = logger.into_writer().into_reader();
        println!(
            "reader len: {}, avg bits per reading: {}",
            reader.len(),
            (reader.len() * 8) as f32 / readings.len() as f32
        );

        let mut log_reader = DeltaLoggerReader::<
            BootTimestamp,
            ADCReading<Volt, BootTimestamp>,
            _,
            TimestampFac,
        >::new(reader);
        loop {
            match log_reader.read().await.unwrap() {
                Some(reading) => {
                    println!("{:?}", reading);
                }
                None => break,
            }
        }
    }
}
