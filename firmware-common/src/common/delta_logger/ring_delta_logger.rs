use core::cell::RefCell;
use core::mem::replace;
use core::mem::size_of;

use super::delta_logger::{DeltaLogger, DeltaLoggerReader, UnixTimestampLog};
use super::delta_logger_trait::BackgroundRunDeltaLoggerTrait;
use super::prelude::DeltaLoggerTrait;
use crate::common::delta_logger::bitslice_primitive::BitSlicePrimitive;
use crate::common::{
    delta_logger::bitslice_serialize::BitArraySerializable, variable_int::VariableIntTrait,
};
use crate::driver::{clock::Clock, delay::Delay, timestamp::BootTimestamp};
use crate::common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
        ticker::Ticker,
    };
use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex, signal::Signal};
use vlfs::{ConcurrentFilesIterator, Crc, FileType, Flash, VLFSError, VLFS};

pub struct RingDeltaLoggerConfig {
    pub file_type: FileType,
    pub seconds_per_segment: u32,
    pub first_segment_seconds: u32,
    pub segments_per_ring: u32,
}

pub struct RingDeltaLogger<'a, D, C, F, FF, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    fs: &'a VLFS<F, C>,
    delta_logger: Mutex<NoopRawMutex, Option<DeltaLogger<D, vlfs::FileWriter<'a, F, C>, FF>>>,
    close_signal: Signal<NoopRawMutex, ()>,
    delay: DL,
    clock: CL,
    config: RingDeltaLoggerConfig,
    current_ring_segments: RefCell<u32>,
}

impl<'a, D, C, F, FF, DL, CL> RingDeltaLogger<'a, D, C, F, FF, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        delay: DL,
        clock: CL,
        config: RingDeltaLoggerConfig,
    ) -> Result<Self, VLFSError<F::Error>> {
        let mut files_iter = fs.files_iter(config.file_type).await;
        let mut files_count = 0;
        while let Some(_) = files_iter.next().await? {
            files_count += 1;
        }
        drop(files_iter);
        log_info!("Found {} files", files_count);

        let mut builder = fs.new_at_builder().await?;

        let (mut files_to_remove, current_ring_segments) = if files_count > config.segments_per_ring - 1
        {
            (
                files_count - config.segments_per_ring + 1,
                config.segments_per_ring,
            )
        } else {
            (0, files_count + 1)
        };

        log_info!("Removing {} extra files", files_to_remove);
        while let Some(file_entry) = builder.read_next().await? {
            if file_entry.typ == config.file_type && files_to_remove > 0 {
                files_to_remove -= 1;
                builder.release_file_sectors(&file_entry).await?;
            } else {
                builder.write(&file_entry).await?;
            }
        }

        let writer = builder
            .write_new_file_and_open_for_write(config.file_type)
            .await?;
        builder.commit().await?;

        let delta_logger = DeltaLogger::new(writer);

        Ok(Self {
            fs,
            delay,
            clock,
            delta_logger: Mutex::new(Some(delta_logger)),
            close_signal: Signal::new(),
            config,
            current_ring_segments: RefCell::new(current_ring_segments),
        })
    }

    async fn create_new_segment(&self) -> Result<(), VLFSError<F::Error>> {
        log_info!("Creating new ring segment");
        let mut builder = self.fs.new_at_builder().await?;
        let new_ring_segments =
            if *self.current_ring_segments.borrow() >= self.config.segments_per_ring {
                let mut first_segment_removed = false;
                while let Some(file_entry) = builder.read_next().await? {
                    if file_entry.typ == self.config.file_type && !first_segment_removed {
                        log_info!("Deleting one ring segment");
                        first_segment_removed = true;
                        builder.release_file_sectors(&file_entry).await?;
                    } else {
                        builder.write(&file_entry).await?;
                    }
                }
                self.config.segments_per_ring
            } else {
                while let Some(file_entry) = builder.read_next().await? {
                    builder.write(&file_entry).await?;
                }
                *self.current_ring_segments.borrow() + 1
            };
        let new_writer = builder
            .write_new_file_and_open_for_write(self.config.file_type)
            .await?;
        builder.commit().await?;
        let new_delta_logger = DeltaLogger::new(new_writer);
        let mut old_delta_logger = {
            let mut delta_logger = self.delta_logger.lock().await;
            let delta_logger = delta_logger.as_mut().unwrap();
            replace(delta_logger, new_delta_logger)
        };

        old_delta_logger.flush().await.map_err(|e|e.0)?;
        let old_writer = old_delta_logger.into_inner().await.map_err(|e|e.0)?;
        old_writer.close().await?;
        *self.current_ring_segments.borrow_mut() = new_ring_segments;

        log_info!("Ring segment created");
        Ok(())
    }

    pub fn log_stats(&self) {
        let readings_per_segment =
            (self.config.seconds_per_segment as f64 * 1000.0 / FF::min()) as u32;
        let reading_size_bits = D::len_bits() + 64 + 2;
        let reading_delta_size_bits =
            D::DeltaType::len_bits() + <FF::VI as VariableIntTrait>::Packed::len_bits() + 2;

        // assume worst case 30% of readings are full readings
        let avg_reading_size_bits =
            0.7 * reading_size_bits as f32 + 0.3 * reading_delta_size_bits as f32;
        let segment_size_kb = (readings_per_segment as f32 * avg_reading_size_bits) / 8.0 / 1024.0;
        let ring_size_kb = segment_size_kb * self.config.segments_per_ring as f32;
        log_info!(
            "segment size: {}kb, ring size: {}kb",
            segment_size_kb as u32,
            ring_size_kb as u32
        );
    }
}


impl<'a, D, C, F, FF, DL, CL> DeltaLoggerTrait<D, ()> for &RingDeltaLogger<'a, D, C, F, FF, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    type Error = VLFSError<F::Error>;

    async fn log(&mut self, reading: SensorReading<BootTimestamp, D>) -> Result<bool, Self::Error> {
        let mut delta_logger = self.delta_logger.lock().await;
        let delta_logger = delta_logger.as_mut().unwrap();
        delta_logger.log(reading).await.map_err(|e|e.0)
    }

    async fn log_unix_time(&mut self, log: UnixTimestampLog) -> Result<(), Self::Error> {
        let mut delta_logger = self.delta_logger.lock().await;
        let delta_logger = delta_logger.as_mut().unwrap();
        delta_logger.log_unix_time(log).await.map_err(|e|e.0)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        let mut delta_logger = self.delta_logger.lock().await;
        let delta_logger = delta_logger.as_mut().unwrap();
        delta_logger.flush().await.map_err(|e|e.0)
    }

    async fn into_inner(self) -> Result<(), Self::Error> {
        self.close_signal.signal(());
        Ok(())
    }
}

impl<'a, D, C, F, FF, DL, CL> BackgroundRunDeltaLoggerTrait<D, ()> for &RingDeltaLogger<'a, D, C, F, FF, DL, CL>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    DL: Delay,
    CL: Clock,
    [(); size_of::<D>() + 10]:,
{
    async fn run(&mut self) -> Result<(), Self::Error> {
        self.delay
            .delay_ms(self.config.first_segment_seconds as f64 * 1000.0)
            .await;
        self.create_new_segment().await?;

        let mut ticker = Ticker::every(
            self.clock.clone(),
            self.delay.clone(),
            self.config.seconds_per_segment as f64 * 1000.0,
        );
        loop {
            match select(ticker.next(), self.close_signal.wait()).await {
                Either::First(_) => {
                    self.create_new_segment().await?;
                }
                Either::Second(_) => {
                    let mut delta_logger = self.delta_logger.lock().await;
                    let mut delta_logger = delta_logger.take().unwrap();
                    delta_logger.flush().await.map_err(|e|e.0)?;
                    let writer = delta_logger.into_inner().await.map_err(|e|e.0)?;
                    writer.close().await?;
                    return Ok(());
                }
            }
        }
    }
}

pub struct RingDeltaLoggerReader<'a, D, C, F, FF>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    fs: &'a VLFS<F, C>,
    file_iter: ConcurrentFilesIterator<'a, F, C, FileType>,
    delta_logger_reader: Option<DeltaLoggerReader<D, vlfs::FileReader<'a, F, C>, FF>>,
}

enum DeltaLoggerReaderResult<D>
where
    D: SensorData,
{
    EOF,
    Data(either::Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>),
    TryAgain,
}

impl<'a, D, C, F, FF> RingDeltaLoggerReader<'a, D, C, F, FF>
where
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    [(); size_of::<D>() + 10]:,
{
    pub async fn new(fs: &'a VLFS<F, C>, file_type: FileType) -> Result<Self, VLFSError<F::Error>> {
        let mut file_iter = fs.concurrent_files_iter(file_type).await;

        if let Some(first_file) = file_iter.next().await? {
            let file_reader = fs.open_file_for_read(first_file.id).await?;
            let delta_logger_reader = DeltaLoggerReader::new(file_reader);
            return Ok(RingDeltaLoggerReader {
                fs,
                file_iter,
                delta_logger_reader: Some(delta_logger_reader),
            });
        } else {
            return Ok(RingDeltaLoggerReader {
                fs,
                file_iter,
                delta_logger_reader: None,
            });
        }
    }

    async fn inner_read(&mut self) -> Result<DeltaLoggerReaderResult<D>, VLFSError<F::Error>> {
        if self.delta_logger_reader.is_none() {
            if let Some(file) = self.file_iter.next().await? {
                let file_reader = self.fs.open_file_for_read(file.id).await?;
                self.delta_logger_reader = Some(DeltaLoggerReader::new(file_reader));
            }
        }

        if let Some(delta_logger_reader) = &mut self.delta_logger_reader {
            let reading = delta_logger_reader.read().await?;
            if let Some(reading) = reading {
                return Ok(DeltaLoggerReaderResult::Data(reading));
            } else {
                let reader = self.delta_logger_reader.take().unwrap();
                let reader = reader.into_reader();
                reader.close().await;
                return Ok(DeltaLoggerReaderResult::TryAgain);
            }
        } else {
            return Ok(DeltaLoggerReaderResult::EOF);
        }
    }

    pub async fn read(
        &mut self,
    ) -> Result<
        Option<either::Either<SensorReading<BootTimestamp, D>, UnixTimestampLog>>,
        VLFSError<F::Error>,
    > {
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
}
