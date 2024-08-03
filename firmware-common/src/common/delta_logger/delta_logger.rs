use core::marker::PhantomData;

use either::Either;

use crate::{
    common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
        variable_int::VariableIntTrait,
    },
    driver::timestamp::BootTimestamp,
};

use super::bitslice_serialize::{BitArraySerializable, BitSliceReader, BitSliceWriter};
use super::delta_factory::{DeltaFactory, Deltable, UnDeltaFactory};
use crate::common::delta_logger::bitslice_primitive::BitSlicePrimitive;

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum Header {
    DeltaTimestampDeltaData,
    DeltaTimestampFullData,
    UnixTimeStampLog,
    EndOfByte,
    FullTimestampFullData,
}

impl Header {
    // Using huffman coding:
    // 10 -> delta timestamp, delta data
    // 01 -> delta timestamp, full data
    // 00 -> unix timestamp log
    // 111 -> end of byte
    // 110 -> full timestamp, full data
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        match self {
            Header::DeltaTimestampDeltaData => writer.write([true, false]),
            Header::DeltaTimestampFullData => writer.write([false, true]),
            Header::UnixTimeStampLog => writer.write([false, false]),
            Header::EndOfByte => writer.write([true, true, true]),
            Header::FullTimestampFullData => writer.write([true, true, false]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnixTimestampLog {
    pub boot_timestamp: f64,
    pub unix_timestamp: f64,
}

impl BitArraySerializable for UnixTimestampLog {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.boot_timestamp);
        writer.write(self.unix_timestamp);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            boot_timestamp: reader.read().unwrap(),
            unix_timestamp: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64 + 64
    }
}

/// If the readings are closer than the minimum value supported by the
/// fixed point factory, they will be ignored
pub struct DeltaLogger<D, W, FF>
where
    D: SensorData,
    W: embedded_io_async::Write,
    FF: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    factory: DeltaFactory<D>,
    timestamp_factory: DeltaFactory<Timestamp<FF>>,
    writer: W,
    bit_writer: BitSliceWriter<{ size_of::<D>() + 10 }>,
    last_entry_is_unix_time: bool,
    unix_time_log_buffer: Option<UnixTimestampLog>,
}

impl<D, W, FF> DeltaLogger<D, W, FF>
where
    D: SensorData,
    W: embedded_io_async::Write,
    FF: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    pub fn new(writer: W) -> Self {
        Self {
            factory: DeltaFactory::new(),
            timestamp_factory: DeltaFactory::new(),
            writer,
            bit_writer: Default::default(),
            last_entry_is_unix_time: false,
            unix_time_log_buffer: None,
        }
    }

    /// returns true if the reading was logged
    pub async fn log(
        &mut self,
        reading: SensorReading<BootTimestamp, D>,
    ) -> Result<bool, W::Error> {
        if let Some(last_timestamp) = &self.timestamp_factory.last_value {
            let interval = reading.timestamp - last_timestamp.0;
            if interval < FF::min() {
                return Ok(false);
            }
        }

        if let Some(unix_time_log) = self.unix_time_log_buffer.take() {
            self.log_unix_time(unix_time_log).await?;
        }

        match self.timestamp_factory.push(reading.timestamp.into()) {
            Either::Left(full_timestamp) => {
                Header::FullTimestampFullData.serialize(&mut self.bit_writer);
                self.bit_writer.write(full_timestamp.0);
                reading.data.serialize(&mut self.bit_writer);
                self.factory.push_no_delta(reading.data);
            }
            Either::Right(delta_timestamp) => match self.factory.push(reading.data) {
                Either::Left(data) => {
                    Header::DeltaTimestampFullData.serialize(&mut self.bit_writer);
                    self.bit_writer.write(delta_timestamp.0);
                    data.serialize(&mut self.bit_writer);
                }
                Either::Right(delta) => {
                    Header::DeltaTimestampDeltaData.serialize(&mut self.bit_writer);
                    self.bit_writer.write(delta_timestamp.0);
                    delta.serialize(&mut self.bit_writer);
                }
            },
        }

        self.writer
            .write_all(self.bit_writer.view_full_byte_slice())
            .await?;
        self.bit_writer.clear_full_byte_slice();

        self.last_entry_is_unix_time = false;
        Ok(true)
    }

    pub async fn log_unix_time(&mut self, log: UnixTimestampLog) -> Result<(), W::Error> {
        if self.last_entry_is_unix_time {
            self.unix_time_log_buffer = Some(log);
            return Ok(());
        }

        Header::UnixTimeStampLog.serialize(&mut self.bit_writer);
        log.serialize(&mut self.bit_writer);
        self.writer
            .write_all(self.bit_writer.view_full_byte_slice())
            .await?;
        self.bit_writer.clear_full_byte_slice();

        self.last_entry_is_unix_time = true;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), W::Error> {
        Header::EndOfByte.serialize(&mut self.bit_writer);
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

pub struct DeltaLoggerReader<D, R, FF>
where
    D: SensorData,
    R: embedded_io_async::Read,
    FF: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    factory: UnDeltaFactory<D>,
    timestamp_factory: UnDeltaFactory<Timestamp<FF>>,
    reader: R,
    bit_reader: BitSliceReader<{ size_of::<D>() + 10 }>,
}

enum DeltaLoggerReaderResult<D>
where
    D: SensorData,
{
    EOF,
    Data(Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>),
    TryAgain,
}

impl<D, R, F> DeltaLoggerReader<D, R, F>
where
    D: SensorData,
    R: embedded_io_async::Read,
    F: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    pub fn new(reader: R) -> Self {
        Self {
            factory: UnDeltaFactory::new(),
            timestamp_factory: UnDeltaFactory::new(),
            reader,
            bit_reader: Default::default(),
        }
    }

    pub async fn read(
        &mut self,
    ) -> Result<Option<Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>>, R::Error> {
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

    async fn inner_read(&mut self) -> Result<DeltaLoggerReaderResult<D>, R::Error> {
        if !self.ensure_at_least_bits(2).await? {
            return Ok(DeltaLoggerReaderResult::EOF);
        }

        let header_1: bool = self.bit_reader.read().unwrap();
        let header_2: bool = self.bit_reader.read().unwrap();
        let header = match (header_1, header_2) {
            (true, false) => Header::DeltaTimestampDeltaData,
            (false, true) => Header::DeltaTimestampFullData,
            (false, false) => Header::UnixTimeStampLog,
            (true, true) => {
                if !self.ensure_at_least_bits(1).await? {
                    return Ok(DeltaLoggerReaderResult::EOF);
                }
                if self.bit_reader.read().unwrap() {
                    Header::EndOfByte
                } else {
                    Header::FullTimestampFullData
                }
            }
        };

        if header == Header::EndOfByte {
            self.bit_reader.skip_byte();
            return Ok(DeltaLoggerReaderResult::TryAgain);
        }

        if header == Header::UnixTimeStampLog {
            if !self.ensure_at_least_bits(UnixTimestampLog::len_bits()).await? {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            let log = UnixTimestampLog::deserialize(&mut self.bit_reader);
            return Ok(DeltaLoggerReaderResult::Data(Either::Right(log)));
        }

        let (is_full_timestamp, is_full_data) = match header {
            Header::FullTimestampFullData => (true, true),
            Header::DeltaTimestampDeltaData => (false, false),
            Header::DeltaTimestampFullData => (false, true),
            _ => {
                log_unreachable!()
            }
        };

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
            if !self.ensure_at_least_bits(D::len_bits()).await? {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            let data = self.factory.push(D::deserialize(&mut self.bit_reader));
            Some(data)
        } else {
            if !self.ensure_at_least_bits(D::DeltaType::len_bits()).await? {
                return Ok(DeltaLoggerReaderResult::EOF);
            }
            self.factory
                .push_delta(D::DeltaType::deserialize(&mut self.bit_reader))
        };

        if timestamp.is_none() || data.is_none() {
            return Ok(DeltaLoggerReaderResult::TryAgain);
        }

        let timestamp = timestamp.unwrap();
        let data = data.unwrap();

        Ok(DeltaLoggerReaderResult::Data(Either::Left(
            SensorReading::new(timestamp.0, data),
        )))
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

    use super::*;
    use crate::{
        common::{sensor_reading::SensorReading, test_utils::BufferWriter},
        driver::{adc::ADCData, timestamp::BootTimestamp},
        fixed_point_factory, Volt,
    };
    use approx::assert_relative_eq;
    use rand::Rng;

    #[tokio::test]
    async fn test_delta_logger_write_read() {
        fixed_point_factory!(TimestampFac, f64, 0.0, 510.0, 0.5);
        let mut readings = vec![
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(0.0, ADCData::new(0.0)),
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(501.0, ADCData::new(0.05)), // inside delta range
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(1000.0, ADCData::new(-0.05)), // inside delta range
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(1501.0, ADCData::new(-0.02)), // inside delta range
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(2000.0, ADCData::new(1.0)), // outside delta range
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(2501.0, ADCData::new(1.07)), // inside delta range
            SensorReading::<BootTimestamp, ADCData<Volt>>::new(5000.0, ADCData::new(1.1)), // timestamp outside delta range
        ];

        let mut rng = rand::thread_rng();
        let mut timestamp = 5000.0f64;
        let mut data = 1.1f32;
        for _ in 0..150 {
            timestamp += rng.gen_range(1.0..500.0);
            data += rng.gen_range(-0.1..0.1);
            readings.push(SensorReading::<BootTimestamp, ADCData<Volt>>::new(
                timestamp,
                ADCData::new(data),
            ))
        }

        let mut buffer = [0u8; 512];
        let writer = BufferWriter::new(&mut buffer);
        let mut logger = DeltaLogger::<_, _, TimestampFac>::new(writer);
        for reading in readings.iter() {
            logger.log(reading.clone()).await.unwrap();
        }
        logger.flush().await.unwrap();

        let writer = logger.into_writer();
        let reader = writer.into_reader();
        println!(
            "reader len: {}, avg bits per reading: {}",
            reader.len(),
            (reader.len() * 8) as f32 / readings.len() as f32
        );

        let mut log_reader = DeltaLoggerReader::<ADCData<Volt>, _, TimestampFac>::new(reader);
        let mut i = 0usize;
        loop {
            match log_reader.read().await.unwrap() {
                Some(reading) => {
                    println!("{:?}", reading);
                    let reading = reading.unwrap_left();
                    assert_relative_eq!(reading.timestamp, readings[i].timestamp, epsilon = 0.5);
                    assert_relative_eq!(reading.data.value, readings[i].data.value, epsilon = 0.1);
                }
                None => break,
            }
            i += 1;
        }
        assert_eq!(i, readings.len());
    }
}
