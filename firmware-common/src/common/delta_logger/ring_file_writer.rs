use core::cell::RefCell;
use core::{mem::replace, ops::DerefMut};

use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex},
    mutex::{Mutex, MutexGuard},
    signal::Signal,
};
use vlfs::{Crc, FileWriter, Flash, VLFSError, VLFS};

use crate::common::ticker::Ticker;
use crate::driver::clock::Clock;
use crate::driver::delay::Delay;

use super::prelude::RingDeltaLoggerConfig;

pub struct RingFileWriter<'a, F, C, DL, CL>
where
    C: Crc,
    F: Flash,
    DL: Delay,
    CL: Clock,
{
    fs: &'a VLFS<F, C>,
    current_writer: Mutex<NoopRawMutex, Option<vlfs::FileWriter<'a, F, C>>>,
    current_writer_dirty: BlockingMutex<NoopRawMutex, RefCell<bool>>,
    close_signal: Signal<NoopRawMutex, ()>,
    config: RingDeltaLoggerConfig,
    delay: DL,
    clock: CL,
    current_ring_segments: BlockingMutex<NoopRawMutex, RefCell<u32>>,
    new_segment: BlockingMutex<NoopRawMutex, RefCell<bool>>,
}

impl<'a, F, C, DL, CL> RingFileWriter<'a, F, C, DL, CL>
where
    C: Crc,
    F: Flash,
    DL: Delay,
    CL: Clock,
{
    pub async fn new(
        fs: &'a VLFS<F, C>,
        config: RingDeltaLoggerConfig,
        delay: DL,
        clock: CL,
    ) -> Result<Self, VLFSError<F::Error>> {
        let mut files_iter = fs.files_iter(config.file_type).await;
        let mut files_count = 0;
        while let Some(_) = files_iter.next().await? {
            files_count += 1;
        }
        drop(files_iter);
        log_info!("Found {} files", files_count);

        let mut builder = fs.new_at_builder().await?;
        let (mut files_to_remove, current_ring_segments) = if files_count > config.segments_per_ring
        {
            (
                files_count - config.segments_per_ring,
                config.segments_per_ring,
            )
        } else {
            (0, files_count)
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

        Ok(Self {
            fs,
            current_writer: Mutex::new(Some(writer)),
            current_writer_dirty: BlockingMutex::new(RefCell::new(false)),
            close_signal: Signal::new(),
            config,
            delay,
            clock,
            current_ring_segments: BlockingMutex::new(RefCell::new(current_ring_segments)),
            new_segment: BlockingMutex::new(RefCell::new(true)),
        })
    }

    pub async fn get_writer(
        &self,
    ) -> (
        bool,
        MutexGuard<'_, NoopRawMutex, Option<FileWriter<'a, F, C>>>,
    ) {
        self.current_writer_dirty
            .lock(|c: &RefCell<bool>| c.replace(true));
        (
            self.new_segment.lock(|c| c.replace(false)),
            self.current_writer.lock().await,
        )
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
                Either::First(_) => {
                    self.create_new_segment().await?;
                }
                Either::Second(_) => {
                    let mut writer = self.current_writer.lock().await;
                    if let Some(writer) = writer.take() {
                        writer.close().await?;
                    }
                    return Ok(());
                }
            }
        }
    }

    async fn create_new_segment(&self) -> Result<(), VLFSError<F::Error>> {
        if self.current_writer_dirty.lock(|c| !c.borrow().clone()) {
            log_info!("No new data, skipping segment creation");
            return Ok(());
        }

        log_info!("Creating new ring segment");
        let mut builder = self.fs.new_at_builder().await?;
        let new_ring_segments: u32 = if self
            .current_ring_segments
            .lock(|c| *c.borrow() >= self.config.segments_per_ring)
        {
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
            self.current_ring_segments.lock(|c| *c.borrow() + 1)
        };
        let new_writer = builder
            .write_new_file_and_open_for_write(self.config.file_type)
            .await?;
        builder.commit().await?;

        let old_writer = {
            let mut writer = self.current_writer.lock().await;
            replace(writer.deref_mut(), Some(new_writer))
        };
        old_writer.unwrap().close().await?;

        self.current_ring_segments
            .lock(|c| c.replace(new_ring_segments));

        self.new_segment.lock(|c| c.replace(true));
        self.current_writer_dirty.lock(|c| c.replace(false));
        log_info!("Ring segment created");
        Ok(())
    }
}
