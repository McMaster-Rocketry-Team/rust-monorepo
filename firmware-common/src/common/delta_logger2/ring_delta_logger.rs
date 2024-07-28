use core::mem::replace;

use vlfs::{Crc, FileType, Flash, VLFSError, VLFS};

use crate::{
    common::{fixed_point::F64FixedPointFactory, sensor_reading::SensorReading},
    driver::timestamp::TimestampType,
};

use super::delta_logger::DeltaLogger;

pub struct RingDeltaLogger<'a, TM, T, C, F, FF>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    T: SensorReading<TM>,
    FF: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
{
    fs: &'a VLFS<F, C>,
    file_type: FileType,
    delta_logger: DeltaLogger<TM, T, vlfs::FileWriter<'a, F, C>, FF>,
    logs_per_segment: u32,
    current_segment_logs: u32,
    segments_per_ring: u32,
    current_ring_segments: u32,
}

impl<'a, TM, T, C, F, FF> RingDeltaLogger<'a, TM, T, C, F, FF>
where
    TM: TimestampType,
    C: Crc,
    F: Flash,
    F::Error: defmt::Format,
    T: SensorReading<TM>,
    FF: F64FixedPointFactory,
    [(); size_of::<T::Data>() + 10]:,
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

        let mut builder = fs.new_at_builder().await?;

        let (mut files_to_remove, current_ring_segments) = if files_count > segments_per_ring {
            (files_count - segments_per_ring, segments_per_ring)
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
            delta_logger,
            logs_per_segment,
            current_segment_logs: 0,

            segments_per_ring,
            current_ring_segments,
        })
    }

    pub async fn log(&mut self, value: T) -> Result<(), VLFSError<F::Error>> {
        if self.current_segment_logs >= self.logs_per_segment {
            log_info!("Creating new ring segment");
            let mut builder = self.fs.new_at_builder().await?;

            let new_ring_segments= if self.current_ring_segments >= self.segments_per_ring {
                let mut first_segment_removed = false;
                while let Some(file_entry) = builder.read_next().await? {
                    if file_entry.typ == self.file_type && !first_segment_removed {
                        first_segment_removed = true;
                    } else {
                        builder.write(&file_entry).await?;
                    }
                }
                self.segments_per_ring
            } else {
                while let Some(file_entry) = builder.read_next().await? {
                    builder.write(&file_entry).await?;
                }
                self.current_ring_segments +1
            };

            let new_writer = builder
                .write_new_file_and_open_for_write(self.file_type)
                .await?;
            builder.commit().await?;

            let new_delta_logger = DeltaLogger::new(new_writer);
            let mut old_delta_logger = replace(&mut self.delta_logger, new_delta_logger);

            old_delta_logger.flush().await?;
            let old_writer = old_delta_logger.into_writer();
            old_writer.close().await?;

            self.current_segment_logs = 0;
            self.current_ring_segments = new_ring_segments;
        }

        let logged = self.delta_logger.log(value).await?;
        self.current_segment_logs += logged as u32;

        Ok(())
    }

    pub async fn close(mut self)  -> Result<(), VLFSError<F::Error>> {
        self.delta_logger.flush().await?;
        let writer = self.delta_logger.into_writer();
        writer.close().await?;
        Ok(())
    }
}
