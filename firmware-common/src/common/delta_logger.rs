use core::mem::{replace, size_of};

use super::delta_factory::{DeltaFactory, Deltable, UnDeltaFactory};
use async_iterator::Iterator;
use defmt::info;
use either::Either;
use rkyv::ser::serializers::BufferSerializer;
use rkyv::ser::Serializer;
use rkyv::{archived_root, Archive, Deserialize, Serialize};
use vlfs::{Crc, FileEntry, FileID, FileType, Flash, VLFSError, VLFS};

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
        let mut files_iter = fs.files_iter().await;
        let mut files_count = 0;
        while let Some(file) = files_iter.next().await {
            if matches!(file, Ok(FileEntry {typ, ..}) if typ == file_type) {
                files_count += 1;
            }
        }
        let current_segment_i = if files_count > segments_per_ring {
            let mut files_to_remove = files_count - segments_per_ring;
            info!("Removing {} extra files", files_to_remove);
            let mut files_iter = fs.files_iter().await;
            while files_to_remove > 0
                && let Some(file) = files_iter.next().await
            {
                if matches!(file, Ok(FileEntry {typ, ..}) if typ == file_type) {
                    let file = file.unwrap();
                    fs.remove_file(file.id).await?;
                    files_to_remove -= 1;
                }
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
                let mut files_iter = self.fs.files_iter().await;
                while let Some(file) = files_iter.next().await {
                    if matches!(file, Ok(FileEntry {typ, ..}) if typ == self.file_type) {
                        let file = file.unwrap();
                        self.fs.remove_file(file.id).await?;
                        break;
                    }
                }
            }
        }

        self.delta_logger.log(value).await?;
        self.current_segment_logs += 1;
        Ok(())
    }

    pub async fn close(mut self) -> Result<(), VLFSError<F::Error>> {
        let file_writer = self.delta_logger.into_writer();
        file_writer.close().await?;
        Ok(())
    }
}

pub struct RingDeltaLogReader<'a, T, C: Crc, F: Flash>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    [(); size_of::<T::Archived>()]:,
{
    fs: &'a VLFS<F, C>,
    file_type: FileType,
    delta_log_reader: DeltaLogReader<T, vlfs::FileReader<'a, F, C>>,
    current_file_id: Option<FileID>,
}

impl<'a, T, C: Crc, F: Flash> RingDeltaLogReader<'a, T, C, F>
where
    F::Error: defmt::Format,
    T: Deltable,
    T: Archive,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    T::DeltaType: Archive,
    <<T as Deltable>::DeltaType as Archive>::Archived: Deserialize<T::DeltaType, rkyv::Infallible>,
    [(); size_of::<T::Archived>()]:,
{
    pub async fn new(fs: &'a VLFS<F, C>, file_type: FileType) -> Result<Self, VLFSError<F::Error>> {
        let mut files_iter = fs.files_iter().await;
        let mut current_file_id = None;
        while let Some(file) = files_iter.next().await {
            if let Ok(FileEntry { typ, id, .. }) = file {
                if typ == file_type {
                    current_file_id = Some(id);
                    break;
                }
            }
        }
        // let current_file_id = current_file_id.ok_or(VLFSError::FileNotFound)?;

        // let file_reader = fs.open_file_for_read(current_file_id).await?;
        // let delta_log_reader = DeltaLogReader::new(file_reader);
        // Ok(Self {
        //     fs,
        //     file_type,
        //     delta_log_reader,
        //     current_file_id,
        // })

        todo!()
    }
}
