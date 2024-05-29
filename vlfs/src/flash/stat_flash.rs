use crate::Flash;
use core::cell::RefCell;
use defmt::Format;
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};
use paste::paste;

use crate::Timer;

#[derive(Debug, Format, Clone, Default)]
pub struct Stat {
    pub erase_sector_4kib_total_time_ms: f64,
    pub erase_sector_4kib_count: usize,
    pub erase_block_32kib_total_time_ms: f64,
    pub erase_block_32kib_count: usize,
    pub erase_block_64kib_total_time_ms: f64,
    pub erase_block_64kib_count: usize,
    pub read_4kib_total_time_ms: f64,
    pub read_4kib_count: usize,
    pub write_256b_total_time_ms: f64,
    pub write_256b_count: usize,
}

pub struct StatFlash {
    stat: BlockingMutex<NoopRawMutex, RefCell<Stat>>,
}

impl StatFlash {
    pub fn new() -> Self {
        Self {
            stat: BlockingMutex::new(RefCell::new(Stat::default())),
        }
    }

    pub fn reset_stat(&self) {
        self.stat.lock(|stat| {
            let mut stat = stat.borrow_mut();
            *stat = Stat::default();
        });
    }

    pub fn get_stat(&self) -> Stat {
        self.stat.lock(|stat| {
            let stat = stat.borrow();
            stat.clone()
        })
    }

    pub fn get_flash<'a, F: Flash, T: Timer>(&self, flash: F, timer: T) -> StatFlashFlash<F, T> {
        StatFlashFlash::new(flash, timer, self)
    }
}

pub struct StatFlashFlash<'a, F: Flash, T: Timer> {
    flash: F,
    timer: T,
    stat_flash: &'a StatFlash,
}

impl<'a, F: Flash, T: Timer> StatFlashFlash<'a, F, T> {
    pub fn new(flash: F, timer: T, stat_flash: &'a StatFlash) -> Self {
        Self {
            flash,
            timer,
            stat_flash,
        }
    }
}

macro_rules! run_with_stat {
    ($self:ident, $func:ident, $($arg:expr),*) => {{
        let start = $self.timer.now_ms();
        let result = $self.flash.$func($($arg),*).await;
        let end = $self.timer.now_ms();
        $self.stat_flash.stat.lock(|stat|{
            let mut stat = stat.borrow_mut();
            paste!{
                stat.[< $func _total_time_ms >] += end-start;
                stat.[< $func _count >] += 1;
            }
        });
        result
    }};
}

impl<'a, F: Flash, T: Timer> Flash for StatFlashFlash<'a, F, T> {
    type Error = F::Error;

    async fn size(&self) -> u32 {
        self.flash.size().await
    }

    async fn reset(&mut self) -> Result<(), F::Error> {
        self.flash.reset().await
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), F::Error> {
        run_with_stat!(self, erase_sector_4kib, address)
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), F::Error> {
        run_with_stat!(self, erase_block_32kib, address)
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), F::Error> {
        run_with_stat!(self, erase_block_64kib, address)
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], F::Error> {
        run_with_stat!(self, read_4kib, address, read_length, read_buffer)
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), F::Error> {
        run_with_stat!(self, write_256b, address, write_buffer)
    }
}
