use core::cell::RefCell;
use core::mem::replace;

use embassy_futures::select::Either;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex, signal::Signal};
use vlfs::{ConcurrentFilesIterator, Crc, FileEntry, FileType, Flash, VLFSError, VLFS};

use super::delta_logger::{DeltaLogger, DeltaLoggerReader};
use crate::{
    common::{
        fixed_point::F64FixedPointFactory,
        sensor_reading::{SensorData, SensorReading},
        ticker::Ticker,
    },
    driver::timestamp::TimestampType,
    Clock, Delay,
};
use embassy_futures::select::select;

pub struct RingDeltaLoggerConfig {
    pub seconds_per_segment: u32,
    pub first_segment_seconds: u32,
    pub segments_per_ring: u32,
}

pub struct RingDeltaLogger<'a, TM, D, C, F, FF, DL, CL>
where
    TM: TimestampType,
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
    file_type: FileType,
    delta_logger: Mutex<NoopRawMutex, Option<DeltaLogger<TM, D, vlfs::FileWriter<'a, F, C>, FF>>>,
    close_signal: Signal<NoopRawMutex, ()>,
    delay: DL,
    clock: CL,
    config:RingDeltaLoggerConfig,
    current_ring_segments: RefCell<u32>,
}

impl<'a, TM, D, C, F, FF, DL, CL> RingDeltaLogger<'a, TM, D, C, F, FF, DL, CL>
where
    TM: TimestampType,
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
        file_type: FileType,
        delay: DL,
        clock: CL,
        config:RingDeltaLoggerConfig,
    ) -> Result<Self, VLFSError<F::Error>> {
        let mut files_iter = fs
            .files_iter_filter(|file_entry| file_entry.typ == file_type)
            .await;
        let mut files_count = 0;
        while let Some(_) = files_iter.next().await? {
            files_count += 1;
        }
        drop(files_iter);
        log_info!("Found {} files", files_count);

        let mut builder = fs.new_at_builder().await?;

        let (mut files_to_remove, current_ring_segments) = if files_count > config.segments_per_ring {
            (files_count - config.segments_per_ring, config.segments_per_ring)
        } else {
            (0, files_count)
        };

        log_info!("Removing {} extra files", files_to_remove);
        while let Some(file_entry) = builder.read_next().await? {
            if file_entry.typ == file_type && files_to_remove > 0 {
                files_to_remove -= 1;
            } else {
                builder.write(&file_entry).await?;
            }
        }

        let writer = builder.write_new_file_and_open_for_write(file_type).await?;
        builder.commit().await?;

        let delta_logger = DeltaLogger::new(writer);

        Ok(Self {
            fs,
            file_type,
            delay,
            clock,
            delta_logger: Mutex::new(Some(delta_logger)),
            close_signal: Signal::new(),
            config,
            current_ring_segments: RefCell::new(current_ring_segments),
        })
    }

    pub async fn log(&self, value: SensorReading<TM, D>) -> Result<bool, VLFSError<F::Error>> {
        let mut delta_logger = self.delta_logger.lock().await;
        let delta_logger = delta_logger.as_mut().unwrap();
        let logged = delta_logger.log(value).await?;

        Ok(logged)
    }

    pub fn close(&self) {
        self.close_signal.signal(());
    }

    pub async fn run(&self) -> Result<(), VLFSError<F::Error>> {
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
                Either::First(_) => {}
                Either::Second(_) => {
                    let mut delta_logger = self.delta_logger.lock().await;
                    let mut delta_logger = delta_logger.take().unwrap();
                    delta_logger.flush().await?;
                    let writer = delta_logger.into_writer();
                    writer.close().await?;
                    return Ok(());
                }
            }
            ticker.next().await;
            self.create_new_segment().await?;
        }
    }

    async fn create_new_segment(&self) -> Result<(), VLFSError<F::Error>> {
        log_info!("Creating new ring segment");
        let mut builder = self.fs.new_at_builder().await?;
        let new_ring_segments = if *self.current_ring_segments.borrow() >= self.config.segments_per_ring {
            let mut first_segment_removed = false;
            while let Some(file_entry) = builder.read_next().await? {
                if file_entry.typ == self.file_type && !first_segment_removed {
                    first_segment_removed = true;
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
            .write_new_file_and_open_for_write(self.file_type)
            .await?;
        builder.commit().await?;
        let new_delta_logger = DeltaLogger::new(new_writer);
        let mut old_delta_logger = {
            let mut delta_logger = self.delta_logger.lock().await;
            let delta_logger = delta_logger.as_mut().unwrap();
            replace(delta_logger, new_delta_logger)
        };

        old_delta_logger.flush().await?;
        let old_writer = old_delta_logger.into_writer();
        old_writer.close().await?;
        *self.current_ring_segments.borrow_mut() = new_ring_segments;

        Ok(())
    }
}

pub struct RingDeltaLoggerReader<'a, TM, D, C, F, FF, P>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    P: FnMut(&FileEntry) -> bool,
    [(); size_of::<D>() + 10]:,
{
    fs: &'a VLFS<F, C>,
    file_iter: ConcurrentFilesIterator<'a, F, C, P>,
    delta_logger_reader: Option<DeltaLoggerReader<TM, D, vlfs::FileReader<'a, F, C>, FF>>,
}

enum DeltaLoggerReaderResult<TM, D>
where
    TM: TimestampType,
    D: SensorData,
{
    EOF,
    Data(SensorReading<TM, D>),
    TryAgain,
}

impl<'a, TM, D, C, F, FF, P> RingDeltaLoggerReader<'a, TM, D, C, F, FF, P>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    D: SensorData,
    FF: F64FixedPointFactory,
    P: FnMut(&FileEntry) -> bool,
    [(); size_of::<D>() + 10]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        file_type: FileType,
    ) -> Result<
        RingDeltaLoggerReader<'a, TM, D, C, F, FF, impl FnMut(&FileEntry) -> bool>,
        VLFSError<F::Error>,
    > {
        let mut file_iter = fs
            .concurrent_files_iter_filter(move |file_entry| file_entry.typ == file_type)
            .await;

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

    async fn inner_read(&mut self) -> Result<DeltaLoggerReaderResult<TM, D>, VLFSError<F::Error>> {
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

    pub async fn read(&mut self) -> Result<Option<SensorReading<TM, D>>, VLFSError<F::Error>> {
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
