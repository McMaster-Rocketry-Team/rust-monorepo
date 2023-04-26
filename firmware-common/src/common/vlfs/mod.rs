use core::cell::RefCell;

use crate::common::buffer::WriteBuffer;
use crate::driver::{crc::Crc, flash::SpiFlash};
use crate::new_write_buffer;
use bitvec::prelude::*;
use defmt::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use heapless::Vec;

use self::utils::find_most_common_u16_out_of_4;
use self::writer::WritingQueueEntry;

use super::rwlock::{RwLock, RwLockReadGuard};

mod daemon;
mod init;
mod iter;
pub mod reader;
mod utils;
pub mod writer;

const VLFS_VERSION: u32 = 5;
const SECTORS_COUNT: usize = 16384; // for 512M-bit flash (W25Q512JV)
const SECTOR_SIZE: usize = 4096;
const PAGE_SIZE: usize = 256;
const PAGES_PER_SECTOR: usize = SECTOR_SIZE / PAGE_SIZE;
const MAX_FILES: usize = 256; // can be as large as 2728
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = SECTORS_COUNT / 32;
const MAX_ALLOC_TABLE_LENGTH: usize = 16 + MAX_FILES * 12;
const WRITING_QUEUE_SIZE: usize = 4;

#[derive(Debug, Clone)]
struct FileEntry {
    file_id: u64,
    file_type: u16,
    first_sector_index: Option<u16>, // None means the file is empty
    opened: bool,
}

// serialized size must fit in half a block (32kib)
struct AllocationTable {
    sequence_number: u32,
    file_entries: Vec<FileEntry, MAX_FILES>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            file_entries: Vec::new(),
        }
    }
}

struct AllocationTableWrapper {
    allocation_table: AllocationTable,
    allocation_table_index: usize, // which half block is the allocation table in
}

impl Default for AllocationTableWrapper {
    fn default() -> Self {
        Self {
            allocation_table: AllocationTable::default(),
            allocation_table_index: TABLE_COUNT - 1,
        }
    }
}

pub struct VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    allocation_table: RwLock<CriticalSectionRawMutex, AllocationTableWrapper, 10>,
    free_sectors: BlockingMutex<
        CriticalSectionRawMutex,
        RefCell<BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>>,
    >,
    flash: Mutex<CriticalSectionRawMutex, F>,
    crc: Mutex<CriticalSectionRawMutex, C>,
    writing_queue: Channel<CriticalSectionRawMutex, WritingQueueEntry, WRITING_QUEUE_SIZE>,
}

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub async fn create_file(&self, file_id: u64, file_type: u16) -> Result<(), ()> {
        let mut at = self.allocation_table.write().await;
        for file_entry in &at.allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Err(());
            }
        }

        let file_entry = FileEntry {
            file_id,
            file_type,
            first_sector_index: None,
            opened: false,
        };
        at.allocation_table.file_entries.push(file_entry).unwrap(); // TODO error handling
        drop(at);
        self.write_allocation_table().await;
        Ok(())
    }

    pub async fn remove_file(&self, file_id: u64) -> Result<(), ()> {
        let mut at = self.allocation_table.write().await;
        for i in 0..at.allocation_table.file_entries.len() {
            if at.allocation_table.file_entries[i].file_id == file_id {
                if at.allocation_table.file_entries[i].opened {
                    return Err(());
                }
                at.allocation_table.file_entries.swap_remove(i);
                break;
            }
        }
        drop(at);
        self.write_allocation_table().await;
        // TODO update sectors list
        Ok(())
    }

    pub async fn get_file_size(&self, file_id: u64) -> Option<(usize, usize)> {
        let at = self.allocation_table.read().await;
        if let Some(file_entry) = self.find_file_entry(&at.allocation_table, file_id) {
            let mut size: usize = 0;
            let mut sectors: usize = 0;
            let mut current_sector_index = file_entry.first_sector_index;
            let mut buffer = [0u8; 5 + 8];
            while let Some(sector_index) = current_sector_index {
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;

                // read data length
                let mut flash = self.flash.lock().await;
                flash.read(sector_address, 8, &mut buffer).await;
                size += find_most_common_u16_out_of_4(&buffer[5..13]).unwrap() as usize;

                // read next sector index
                flash
                    .read(sector_address + 4096 - 256, 8, &mut buffer)
                    .await;
                let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
                sectors += 1;
            }

            return Some((size, sectors));
        }

        None
    }

    // TODO optimize
    fn claim_avaliable_sector(&self) -> Option<u16> {
        self.free_sectors.lock(|free_sectors| {
            let mut free_sectors = free_sectors.borrow_mut();
            for i in 0..SECTORS_COUNT {
                if !free_sectors[i] {
                    let slice = free_sectors.as_mut_bitslice();
                    slice.set(i, true);
                    return Some(i.try_into().unwrap());
                }
            }
            None
        })
    }

    fn return_sector(&self, sector_index: u16) {
        self.free_sectors.lock(|free_sectors| {
            let mut free_sectors = free_sectors.borrow_mut();
            let slice = free_sectors.as_mut_bitslice();
            slice.set(sector_index as usize, false);
        })
    }

    fn find_file_entry<'a>(
        &self,
        allocation_table: &'a AllocationTable,
        file_id: u64,
    ) -> Option<&'a FileEntry> {
        for file_entry in &allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Some(file_entry);
            }
        }
        None
    }

    fn find_file_entry_mut<'a>(
        &self,
        allocation_table: &'a mut AllocationTable,
        file_id: u64,
    ) -> Option<&'a mut FileEntry> {
        for file_entry in &mut allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Some(file_entry);
            }
        }
        None
    }
}
