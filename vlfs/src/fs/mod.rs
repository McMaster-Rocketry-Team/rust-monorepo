use core::cell::RefCell;
use core::marker::PhantomData;

use crate::driver::{crc::Crc, flash::Flash};
use bitvec::prelude::*;
use defmt::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};

use self::sector_management::SectorsMng;
use self::{error::VLFSError, utils::find_most_common_u16_out_of_4};
use heapless::Vec;

use crate::utils::rwlock::{RwLock, RwLockReadGuard};

pub mod error;
pub mod init;
pub mod iter;
pub mod reader;
pub mod utils;
pub mod writer;
pub mod sector_management;

const VLFS_VERSION: u32 = 16;
const SECTORS_COUNT: usize = 16384; // for 512M-bit flash (W25Q512JV)
const SECTOR_SIZE: usize = 4096;
const PAGE_SIZE: usize = 256;
const PAGES_PER_SECTOR: usize = SECTOR_SIZE / PAGE_SIZE;
const MAX_FILES: usize = 256; // can be as large as 2728
const TABLE_COUNT: usize = 4;
const MAX_SECTOR_DATA_SIZE: usize = 4016;
const ALLOC_TABLES_SECTORS_USED: usize = TABLE_COUNT * 32 * 1024 / SECTOR_SIZE;
const DATA_REGION_SECTORS: usize = SECTORS_COUNT - ALLOC_TABLES_SECTORS_USED;
const SECTOR_MAP_ARRAY_SIZE: usize = DATA_REGION_SECTORS / 32;

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
    F: Flash,
    C: Crc,
{
    allocation_table: RwLock<CriticalSectionRawMutex, AllocationTableWrapper, 10>,
    sectors_mng: BlockingMutex<CriticalSectionRawMutex, RefCell<SectorsMng>>,
    flash: Mutex<CriticalSectionRawMutex, F>,
    crc: Mutex<CriticalSectionRawMutex, C>,
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn create_file(&self, file_id: u64, file_type: u16) -> Result<(), VLFSError<F>> {
        let mut at = self.allocation_table.write().await;
        for file_entry in &at.allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Err(VLFSError::FileAlreadyExists);
            }
        }

        let file_entry = FileEntry {
            file_id,
            file_type,
            first_sector_index: None,
            opened: false,
        };
        at.allocation_table
            .file_entries
            .push(file_entry)
            .map_err(|_| VLFSError::MaxFilesReached)?;
        drop(at);
        self.write_allocation_table().await?;
        Ok(())
    }

    pub async fn remove_file(&self, file_id: u64) -> Result<(), VLFSError<F>> {
        let mut current_sector_index: Option<u16> = None;
        let mut at = self.allocation_table.write().await;
        for i in 0..at.allocation_table.file_entries.len() {
            if at.allocation_table.file_entries[i].file_id == file_id {
                if at.allocation_table.file_entries[i].opened {
                    return Err(VLFSError::FileInUse);
                }
                current_sector_index = at.allocation_table.file_entries[i].first_sector_index;
                at.allocation_table.file_entries.swap_remove(i);
                break;
            }
        }
        drop(at);
        self.write_allocation_table().await?;

        // update sectors list
        let mut buffer = [0u8; 5 + 8];
        let mut flash = self.flash.lock().await;

        while let Some(sector_index) = current_sector_index {
            let address = sector_index as u32 * SECTOR_SIZE as u32;
            let address = address + SECTOR_SIZE as u32 - 8;

            let read_result = flash
                .read(address, 8, &mut buffer)
                .await
                .map_err(VLFSError::from_flash)?;
            let next_sector_index = find_most_common_u16_out_of_4(read_result).unwrap();
            self.return_sector(sector_index);
            current_sector_index = if next_sector_index == 0xFFFF {
                None
            } else {
                Some(next_sector_index)
            };
        }

        Ok(())
    }

    pub async fn get_file_size(&self, file_id: u64) -> Result<(usize, usize), VLFSError<F>> {
        trace!("get file size start");
        let at = self.allocation_table.read().await;
        if let Some(file_entry) = self.find_file_entry(&at.allocation_table, file_id) {
            let mut size: usize = 0;
            let mut sectors: usize = 0;
            let mut current_sector_index = file_entry.first_sector_index;
            let mut buffer = [0u8; 5 + 16];
            let mut flash = self.flash.lock().await;

            while let Some(sector_index) = current_sector_index {
                let address = sector_index as u32 * SECTOR_SIZE as u32;
                let address = address + SECTOR_SIZE as u32 - 8 - 8;

                let read_result = flash
                    .read(address, 16, &mut buffer)
                    .await
                    .map_err(VLFSError::from_flash)?;

                let sector_data_size =
                    find_most_common_u16_out_of_4(&read_result[..8]).unwrap() as usize; // TODO handle error
                info!(
                    "sector data size = {} at sector #{:#X}",
                    sector_data_size, sector_index
                );
                if sector_data_size > MAX_SECTOR_DATA_SIZE {
                    warn!("sector_data_size > MAX_SECTOR_DATA_SIZE");
                    sectors += 1;
                    break;
                } else {
                    size += sector_data_size;
                }

                let next_sector_index = find_most_common_u16_out_of_4(&read_result[8..]).unwrap();

                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
                sectors += 1;
            }

            return Ok((size, sectors));
        }

        Err(VLFSError::FileDoesNotExist)
    }

    
}


impl<F, C> defmt::Format for VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    fn format(&self, _fmt: Formatter) {
        // TODO
    }
}