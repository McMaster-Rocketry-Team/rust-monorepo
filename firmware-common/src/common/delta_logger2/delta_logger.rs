use core::marker::PhantomData;

use either::Either;

use crate::{
    common::{
        delta_factory::{DeltaFactory, Deltable, UnDeltaFactory},
        fixed_point::F64FixedPointFactory,
        sensor_reading::SensorReading,
        variable_int::VariableIntTrait,
    },
    driver::timestamp::TimestampType,
};

use super::bitslice_io::{BitArraySerializable, BitSliceReader, BitSliceWriter};
use crate::common::delta_logger2::bitslice_io::BitArrayDeserializable;
use crate::common::delta_logger2::bitvec_serialize_traits::FromBitSlice;

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

/// If the readings are closer than the minimum value supported by the
/// fixed point factory, they will be ignored
pub struct DeltaLogger<TM, T, W, FF>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    FF: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    factory: DeltaFactory<T::Data>,
    timestamp_factory: DeltaFactory<Timestamp<FF>>,
    writer: W,
    bit_writer: BitSliceWriter<{ size_of::<T::Data>() + 10 }>,
}

impl<TM, T, W, FF> DeltaLogger<TM, T, W, FF>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    W: embedded_io_async::Write,
    FF: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    pub fn new(
        writer: W,
    ) -> Self {
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
    /// returns true if the reading was logged
    pub async fn log(&mut self, reading: T) -> Result<bool, W::Error> {
        if let Some(last_timestamp) = &self.timestamp_factory.last_value{
            let interval = reading.get_timestamp() - last_timestamp.0;
            if interval < FF::min(){
                return Ok(false);
            }
        }

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

        Ok(true)
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

    /// You need to call flush before calling this method
    pub fn into_writer(self) -> W {
        self.writer
    }
}

pub struct DeltaLoggerReader<TM, T, R, FF>
where
    TM: TimestampType,
    T: SensorReading<TM>,
    R: embedded_io_async::Read,
    FF: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    factory: UnDeltaFactory<T::Data>,
    timestamp_factory: UnDeltaFactory<Timestamp<FF>>,
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

    pub fn into_reader(self) -> R {
        self.reader
    }
}

#[cfg(test)]
mod test {

    use crate::{
        common::test_utils::BufferWriter,
        driver::{
            adc::{ADCData, ADCReading},
            timestamp::BootTimestamp,
        },
        fixed_point_factory2, Volt,
    };

    use super::*;

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
