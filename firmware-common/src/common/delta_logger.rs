use core::cell::RefCell;
use core::mem::{replace, size_of};

use super::delta_factory::{DeltaFactory, Deltable, UnDeltaFactory};
use either::Either;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use rkyv::ser::serializers::BufferSerializer;
use rkyv::ser::Serializer;
use rkyv::{archived_root, Archive, Deserialize, Serialize};
use vlfs::{ConcurrentFilesIterator, Crc, FileEntry, FileType, Flash, VLFSError, VLFS};

pub struct DeltaLogger<T, W>
where
    W: vlfs::AsyncWriter,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    factory: DeltaFactory<T>,
    writer: W,
    buffer: [u8; size_of::<T::Archived>()],
}

impl<T, W> DeltaLogger<T, W>
where
    W: vlfs::AsyncWriter,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    pub fn new(writer: W) -> Self {
        Self {
            factory: DeltaFactory::new(),
            writer,
            buffer: [0; size_of::<T::Archived>()],
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
            Either::Right(delta) => {
                serializer.serialize_value(&delta).unwrap();
                let buffer = serializer.into_inner();
                let buffer = &buffer[..size_of::<<T::DeltaType as Archive>::Archived>()];
                self.writer.extend_from_u8(0x42).await?;
                self.writer.extend_from_slice(buffer).await?;
            }
        };

        Ok(())
    }

    pub fn into_writer(self) -> W {
        self.writer
    }
}

pub struct BufferedDeltaLogger<'a, T, C: Crc, F: Flash, const SIZE: usize>
where
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    logger: RefCell<Option<DeltaLogger<T, vlfs::FileWriter<'a, F, C>>>>,
    queue: Channel<NoopRawMutex, T, SIZE>,
}

impl<'a, T, C: Crc, F: Flash, const SIZE: usize> BufferedDeltaLogger<'a, T, C, F, SIZE>
where
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        file_type: FileType,
    ) -> Result<Self, VLFSError<F::Error>> {
        let file = fs.create_file(file_type).await?;
        let file_writer = fs.open_file_for_write(file.id).await?;
        let logger = DeltaLogger::new(file_writer);
        let queue = Channel::new();
        Ok(Self {
            logger: RefCell::new(Some(logger)),
            queue,
        })
    }

    pub fn log(&self, value: T) {
        let result = self.queue.try_send(value);
        if result.is_err() {
            // log_warn!("Logger queue full");
        }
    }

    pub async fn run(&self) {
        let mut logger = self.logger.borrow_mut().take().unwrap();
        loop {
            let value = self.queue.receive().await;
            logger.log(value).await.ok();
        }
    }
}

pub struct DeltaLogReader<T, R>
where
    R: vlfs::AsyncReader,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    [(); size_of::<T::Archived>()]:,
{
    factory: UnDeltaFactory<T>,
    reader: R,
    buffer: [u8; size_of::<T::Archived>()],
}

impl<T, R> DeltaLogReader<T, R>
where
    R: vlfs::AsyncReader,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    [(); size_of::<T::Archived>()]:,
{
    pub fn new(reader: R) -> Self {
        Self {
            factory: UnDeltaFactory::new(),
            reader,
            buffer: [0; size_of::<T::Archived>()],
        }
    }

    pub async fn next(&mut self) -> Result<Option<T>, R::Error> {
        let (typ, _) = self.reader.read_u8(&mut self.buffer).await?;
        Ok(match typ {
            Some(0x69) => {
                let (slice, _) = self
                    .reader
                    .read_slice(&mut self.buffer, size_of::<T::Archived>())
                    .await?;
                if slice.len() != size_of::<T::Archived>() {
                    return Ok(None);
                }
                let archived = unsafe { archived_root::<T>(&slice) };
                let deserialized: T = archived.deserialize(&mut rkyv::Infallible).unwrap();
                Some(self.factory.push(deserialized))
            }
            Some(0x42) => {
                let length = size_of::<<<T as Deltable>::DeltaType as Archive>::Archived>();
                let (slice, _) = self.reader.read_slice(&mut self.buffer, length).await?;
                if slice.len() != length {
                    return Ok(None);
                }
                let archived = unsafe { archived_root::<T::DeltaType>(&slice) };
                let deserialized: T::DeltaType =
                    archived.deserialize(&mut rkyv::Infallible).unwrap();
                self.factory.push_delta(deserialized)
            }
            _ => None,
        })
    }

    pub fn into_reader(self) -> R {
        self.reader
    }
}

pub struct RingDeltaLogger<'a, T, C: Crc, F: Flash>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    fs: &'a VLFS<F, C>,
    file_type: FileType,
    delta_logger: DeltaLogger<T, vlfs::FileWriter<'a, F, C>>,
    logs_per_segment: u32,
    current_segment_logs: u32,
    segments_per_ring: u32,
    current_segment_i: u32,
}

impl<'a, T, C: Crc, F: Flash> RingDeltaLogger<'a, T, C, F>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        file_type: FileType,
        logs_per_segment: u32,
        segments_per_ring: u32,
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
        let current_segment_i = if files_count > segments_per_ring {
            let mut files_to_remove = files_count - segments_per_ring;
            log_info!("Removing {} extra files", files_to_remove);
            let mut files_iter = fs
                .concurrent_files_iter_filter(|file_entry| file_entry.typ == file_type)
                .await;
            while files_to_remove > 0
                && let Some(file) = files_iter.next().await?
            {
                fs.remove_file(file.id).await?;
                files_to_remove -= 1;
            }
            segments_per_ring
        } else {
            files_count
        };

        let file = fs.create_file(file_type).await?;
        let file_writer = fs.open_file_for_write(file.id).await?;
        let delta_logger = DeltaLogger::new(file_writer);
        Ok(Self {
            fs,
            file_type,
            delta_logger,
            logs_per_segment,
            current_segment_logs: 0,
            segments_per_ring,
            current_segment_i,
        })
    }

    pub async fn log(&mut self, value: T) -> Result<(), VLFSError<F::Error>> {
        if self.current_segment_logs >= self.logs_per_segment {
            log_info!("Creating new ring segment");
            let new_file = self.fs.create_file(self.file_type).await?;
            let new_file_writer = self.fs.open_file_for_write(new_file.id).await?;
            let new_delta_logger = DeltaLogger::new(new_file_writer);

            let old_delta_logger = replace(&mut self.delta_logger, new_delta_logger);
            let old_file_writer = old_delta_logger.into_writer();
            old_file_writer.close().await?;

            self.current_segment_logs = 0;
            self.current_segment_i += 1;

            if self.current_segment_i > self.segments_per_ring {
                log_info!("Removing old ring segment");
                let mut removed = false;
                self.fs
                    .remove_files(|file_entry| {
                        if removed {
                            return false;
                        } else {
                            if file_entry.typ == self.file_type {
                                removed = true;
                                return true;
                            } else {
                                return false;
                            }
                        }
                    })
                    .await?;
            }
            log_info!("Done");
        }

        self.delta_logger.log(value).await?;
        self.current_segment_logs += 1;
        Ok(())
    }

    pub async fn close(self) -> Result<(), VLFSError<F::Error>> {
        let file_writer = self.delta_logger.into_writer();
        file_writer.close().await?;
        Ok(())
    }
}

pub struct RingDeltaLogReader<'a, T, C: Crc, F: Flash, P>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    P: FnMut(&FileEntry) -> bool,
    [(); size_of::<T::Archived>()]:,
{
    fs: &'a VLFS<F, C>,
    file_iter: ConcurrentFilesIterator<'a, F, C, P>,
    delta_log_reader: Option<DeltaLogReader<T, vlfs::FileReader<'a, F, C>>>,
}

impl<'a, T, C: Crc, F: Flash> RingDeltaLogReader<'a, T, C, F, fn(&FileEntry) -> bool>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        file_type: FileType,
    ) -> Result<RingDeltaLogReader<'a, T, C, F, impl FnMut(&FileEntry) -> bool>, VLFSError<F::Error>>
    {
        let mut file_iter = fs
            .concurrent_files_iter_filter(move |file_entry| file_entry.typ == file_type)
            .await;
        if let Some(first_file) = file_iter.next().await? {
            let file_reader = fs.open_file_for_read(first_file.id).await?;
            let delta_log_reader = DeltaLogReader::new(file_reader);
            return Ok(RingDeltaLogReader {
                fs,
                file_iter,
                delta_log_reader: Some(delta_log_reader),
            });
        } else {
            return Ok(RingDeltaLogReader {
                fs,
                file_iter,
                delta_log_reader: None,
            });
        }
    }
}

// FIXME has bug
impl<'a, T, C: Crc, F: Flash, P> RingDeltaLogReader<'a, T, C, F, P>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    P: FnMut(&FileEntry) -> bool,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn next(&mut self) -> Result<Option<T>, VLFSError<F::Error>> {
        if self.delta_log_reader.is_none() {
            if let Some(file) = self.file_iter.next().await? {
                let file_reader = self.fs.open_file_for_read(file.id).await?;
                self.delta_log_reader = Some(DeltaLogReader::new(file_reader));
            }
        }

        if let Some(delta_log_reader) = &mut self.delta_log_reader {
            if let Some(value) = delta_log_reader.next().await? {
                return Ok(Some(value));
            } else {
                self.delta_log_reader
                    .take()
                    .unwrap()
                    .into_reader()
                    .close()
                    .await;
            }
        }

        todo!();
    }
}

pub struct TieredRingDeltaLoggerConfig {
    tier_1_seconds_per_segment: u32,
    tier_1_keep_seconds: u32,
    tier_2_seconds_per_segment: u32,
    tier_2_keep_seconds: u32,
    tier_2_ratio: u32,
}

impl TieredRingDeltaLoggerConfig {
    pub fn new(
        tier_1_seconds_per_segment: u32,
        tier_1_keep_seconds: u32,
        tier_2_seconds_per_segment: u32,
        tier_2_keep_seconds: u32,
    ) -> Self {
        Self {
            tier_1_seconds_per_segment,
            tier_1_keep_seconds,
            tier_2_seconds_per_segment,
            tier_2_keep_seconds,
            tier_2_ratio: tier_1_seconds_per_segment / tier_2_seconds_per_segment,
        }
    }
}

pub struct TieredRingDeltaLogger<'a, 'b, T, C: Crc, F: Flash>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    logger_1: RingDeltaLogger<'a, T, C, F>,
    logger_2: RingDeltaLogger<'a, T, C, F>,
    config: &'b TieredRingDeltaLoggerConfig,
    tier_2_counter: u32,
    max_total_file_size: u32,
}

impl<'a, 'b, T, C: Crc, F: Flash> TieredRingDeltaLogger<'a, 'b, T, C, F>
where
    F::Error: defmt::Format,
    T: Deltable + Clone,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        config: &'b TieredRingDeltaLoggerConfig,
        tier_1_file_type: FileType,
        tier_1_logs_per_second: u32,
        tier_2_file_type: FileType,
        tier_2_logs_per_second: u32,
    ) -> Result<Self, VLFSError<F::Error>> {
        let logger_1 = RingDeltaLogger::new(
            fs,
            tier_1_file_type,
            tier_1_logs_per_second * config.tier_1_seconds_per_segment,
            config.tier_1_keep_seconds / config.tier_1_seconds_per_segment,
        )
        .await?;
        let logger_2 = RingDeltaLogger::new(
            fs,
            tier_2_file_type,
            tier_2_logs_per_second * config.tier_2_seconds_per_segment,
            config.tier_2_keep_seconds / config.tier_2_seconds_per_segment,
        )
        .await?;
        Ok(Self {
            logger_1,
            logger_2,
            config,
            tier_2_counter: 0,
            max_total_file_size: (tier_1_logs_per_second * config.tier_1_keep_seconds
                + tier_2_logs_per_second * config.tier_2_keep_seconds)
                * size_of::<T::Archived>() as u32,
        })
    }

    pub fn max_total_file_size(&self) -> u32 {
        self.max_total_file_size
    }

    pub async fn log(&mut self, value: T) -> Result<(), VLFSError<F::Error>> {
        if self.tier_2_counter == 0 {
            self.logger_2.log(value.clone()).await?;
        }

        self.tier_2_counter += 1;
        if self.tier_2_counter >= self.config.tier_2_ratio {
            self.tier_2_counter = 0;
        }

        self.logger_1.log(value).await?;

        Ok(())
    }

    pub async fn close(self) -> Result<(), VLFSError<F::Error>> {
        self.logger_1.close().await?;
        self.logger_2.close().await?;
        Ok(())
    }
}

pub struct BufferedTieredRingDeltaLogger<'a, 'b, T, C: Crc, F: Flash, const SIZE: usize>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    logger: RefCell<Option<TieredRingDeltaLogger<'a, 'b, T, C, F>>>,
    max_total_file_size: u32,
    queue: Channel<NoopRawMutex, T, SIZE>,
}

impl<'a, 'b, T, C: Crc, F: Flash, const SIZE: usize>
    BufferedTieredRingDeltaLogger<'a, 'b, T, C, F, SIZE>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::DeltaType: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        config: &'b TieredRingDeltaLoggerConfig,
        tier_1_file_type: FileType,
        tier_1_logs_per_second: u32,
        tier_2_file_type: FileType,
        tier_2_logs_per_second: u32,
    ) -> Result<Self, VLFSError<F::Error>> {
        let logger = TieredRingDeltaLogger::new(
            fs,
            config,
            tier_1_file_type,
            tier_1_logs_per_second,
            tier_2_file_type,
            tier_2_logs_per_second,
        )
        .await?;
        let queue = Channel::new();
        Ok(Self {
            max_total_file_size: logger.max_total_file_size(),
            logger: RefCell::new(Some(logger)),
            queue,
        })
    }

    pub fn max_total_file_size(&self) -> u32 {
        self.max_total_file_size
    }

    pub fn log(&self, value: T) {
        let result = self.queue.try_send(value);
        if result.is_err() {
            log_warn!("Logger queue full");
        }
    }

    pub async fn run(&self) {
        let mut logger = self.logger.borrow_mut().take().unwrap();
        loop {
            let value = self.queue.receive().await;
            logger.log(value).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use vlfs::{DummyCrc, DummyFlash};

    use crate::driver::{imu::IMUReading, timestamp::UnixTimestamp};

    use super::*;

    async fn test() {
        let flash = DummyFlash {};
        let crc = DummyCrc {};
        let mut vlfs = VLFS::new(flash, crc);
        vlfs.init().await.unwrap();

        let reader =
            RingDeltaLogReader::<IMUReading<UnixTimestamp>, _, _, _>::new(&vlfs, FileType(0))
                .await
                .unwrap();
    }
}
