use core::mem::{replace, size_of};

use super::delta_factory::{DeltaFactory, Deltable, UnDeltaFactory};
use defmt::info;
use either::Either;
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
        let current_segment_i = if files_count > segments_per_ring {
            let mut files_to_remove = files_count - segments_per_ring;
            info!("Removing {} extra files", files_to_remove);
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
            info!("Creating new ring segment");
            let new_file = self.fs.create_file(self.file_type).await?;
            let new_file_writer = self.fs.open_file_for_write(new_file.id).await?;
            let new_delta_logger = DeltaLogger::new(new_file_writer);

            let old_delta_logger = replace(&mut self.delta_logger, new_delta_logger);
            let old_file_writer = old_delta_logger.into_writer();
            old_file_writer.close().await?;

            self.current_segment_logs = 0;
            self.current_segment_i += 1;

            if self.current_segment_i > self.segments_per_ring {
                info!("Removing old ring segment");
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
