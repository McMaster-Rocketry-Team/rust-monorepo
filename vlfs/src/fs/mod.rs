use core::cell::RefCell;

use crate::driver::{crc::Crc, flash::Flash};
use crate::flash::flash_wrapper::FlashWrapper;
use bitvec::prelude::*;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};

use self::allocation_table::{AllocationTable, FileEntry};
use self::sector_management::SectorsMng;
use self::{error::VLFSError, utils::find_most_common_u16_out_of_4};
use heapless::Vec;

use crate::utils::rwlock::RwLock;

pub mod allocation_table;
pub mod at_builder;
pub mod error;
pub mod hamming;
pub mod init;
pub mod iter;
pub mod reader;
pub mod sector_management;
pub mod utils;
pub mod writer;

const VLFS_VERSION: u32 = 19;
const SECTORS_COUNT: usize = 16384; // for 512M-bit flash (W25Q512JV)
const SECTOR_SIZE: usize = 4096;
const PAGE_SIZE: usize = 256;
const PAGES_PER_SECTOR: usize = SECTOR_SIZE / PAGE_SIZE;
const MAX_DATA_LENGTH_PER_PAGE: usize = PAGE_SIZE - 4;
const MAX_DATA_LENGTH_LAST_PAGE: usize = PAGE_SIZE - 4 - 8 - 8;
const MAX_DATA_LENGTH_PER_SECTION: usize =
    (PAGES_PER_SECTOR - 1) * MAX_DATA_LENGTH_PER_PAGE + MAX_DATA_LENGTH_LAST_PAGE;
const TABLE_COUNT: usize = 4;
const TABLE_SIZE: usize = 32 * 1024;
const MAX_FILES: usize = (TABLE_SIZE - 26 - 4) / 13;
const MAX_SECTOR_DATA_SIZE: usize = 4016;
const ALLOC_TABLES_SECTORS_USED: usize = TABLE_COUNT * TABLE_SIZE / SECTOR_SIZE;
const DATA_REGION_SECTORS: usize = SECTORS_COUNT - ALLOC_TABLES_SECTORS_USED; // must be a multiple of 16 & aligned to 16

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
pub struct FileID(pub u64);

impl From<u64> for FileID {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

#[cfg(feature = "std")]
impl std::hash::Hash for FileID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// File type of 0xFFFF is reserved
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
pub struct FileType(pub u16);

impl From<u16> for FileType {
    fn from(v: u16) -> Self {
        Self(v)
    }
}

pub struct VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    allocation_table: RwLock<NoopRawMutex, AllocationTable, 10>,
    sectors_mng: RwLock<NoopRawMutex, SectorsMng, 10>,
    flash: RwLock<NoopRawMutex, FlashWrapper<F>, 10>,
    crc: Mutex<NoopRawMutex, C>,
    rng: BlockingMutex<NoopRawMutex, RefCell<SmallRng>>,
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub async fn exists(&self, file_id: FileID) -> Result<bool, VLFSError<F::Error>> {
        Ok(self.find_file_entry(file_id).await?.is_some())
    }

    pub async fn remove_file(&self, file_id: FileID) -> Result<(), VLFSError<F::Error>> {
        let (number_of_removed_files, number_of_opened_files_cant_be_removed) = self
            .remove_files(|file_entry| file_entry.id == file_id)
            .await?;
        if number_of_removed_files == 1 {
            return Ok(());
        } else if number_of_opened_files_cant_be_removed == 1 {
            return Err(VLFSError::FileInUse);
        } else {
            return Err(VLFSError::FileDoesNotExist);
        }
    }

    /// returns (number_of_removed_files, number_of_opened_files_cant_be_removed)
    pub async fn remove_files(
        &self,
        mut predicate: impl FnMut(&FileEntry) -> bool,
    ) -> Result<(u16, u16), VLFSError<F::Error>> {
        let mut number_of_removed_files = 0u16;
        let mut unremoved_open_files = 0u16;
        let mut builder = self.new_at_builder().await?;

        while let Some(file_entry) = builder.read_next().await? {
            if predicate(&file_entry) {
                if builder.is_file_opened(file_entry.id) {
                    unremoved_open_files += 1;
                    builder.write(&file_entry).await?;
                } else {
                    number_of_removed_files += 1;
                    builder.release_file_sectors(&file_entry).await?;
                }
            } else {
                builder.write(&file_entry).await?;
            }
        }

        builder.commit().await?;
        Ok((number_of_removed_files, unremoved_open_files))
    }

    /// returns (number_of_removed_files, number_of_opened_files_cant_be_removed)
    pub async fn remove_files_with_type(
        &self,
        file_type: FileType,
    ) -> Result<(u16, u16), VLFSError<F::Error>> {
        self.remove_files(|file_entry| file_entry.typ == file_type)
            .await
    }

    pub async fn get_file_size(
        &self,
        file_id: FileID,
    ) -> Result<(usize, usize), VLFSError<F::Error>> {
        log_trace!("get file size start");
        if let Some((file_entry, _)) = self.find_file_entry(file_id).await? {
            let mut size: usize = 0;
            let mut sectors: usize = 0;
            let mut current_sector_index = file_entry.first_sector_index;
            let mut buffer = [0u8; 5 + 16];
            let flash = self.flash.read().await;

            while let Some(sector_index) = current_sector_index {
                let address = sector_index as u32 * SECTOR_SIZE as u32;
                let address = address + SECTOR_SIZE as u32 - 8 - 8;

                let read_result = flash
                    .read(address, 16, &mut buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;

                let sector_data_size =
                    find_most_common_u16_out_of_4(&read_result[..8]).unwrap() as usize; // TODO handle error
                log_info!(
                    "sector data size = {} at sector #{:#X}",
                    sector_data_size,
                    sector_index
                );
                if sector_data_size > MAX_SECTOR_DATA_SIZE {
                    log_warn!("sector_data_size > MAX_SECTOR_DATA_SIZE");
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

    // This function will return the # of bytes of free space in a vlfs instance in the most optimal situation.
    // Since one sector can be only assigned to one file, If a file is 1kb, it will occupy the entire 4kb sector
    pub async fn free(&self) -> u32 {
        let sectors_mng = self.sectors_mng.read().await;
        let mut free_sector_count = sectors_mng.sector_map.free_sectors_count as u32;
        free_sector_count += sectors_mng.async_erase_ahead_sectors.len() as u32
            + sectors_mng.erase_ahead_sectors.len() as u32;

        (free_sector_count as usize * MAX_SECTOR_DATA_SIZE) as u32
    }

    pub fn into_flash(self) -> F {
        self.flash.into_inner().into_inner()
    }
}
