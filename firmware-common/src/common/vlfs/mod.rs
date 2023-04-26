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
use self::writer::PageWritingQueueEntry;

use super::rwlock::{RwLock, RwLockReadGuard};

mod daemon;
mod init;
mod iter;
mod utils;
mod writer;

const VLFS_VERSION: u32 = 5;
const SECTORS_COUNT: usize = 16384; // for 512M-bit flash (W25Q512JV)
const SECTOR_SIZE: usize = 4096;
const MAX_FILES: usize = 256; // can be as large as 2728
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = SECTORS_COUNT / 32;
const MAX_ALLOC_TABLE_LENGTH: usize = 16 + MAX_FILES * 12;
const MAX_OPENED_FILES: usize = 10;
const WRITING_QUEUE_SIZE: usize = 4;
const MAX_DATA_LENGTH_PER_SECTOR: usize = SECTOR_SIZE - 256 - 8 - 4;

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

#[derive(Debug)]
pub struct OpenedFile {
    current_sector_index: Option<u16>, // None means the file is empty
    file_entry: FileEntry,             // FIXME use reference
}

type FileDescriptor = usize;

pub struct WritingQueueEntry {
    pub fd: FileDescriptor,
    pub data: [u8; 5 + SECTOR_SIZE - 256],
    pub data_length: u16,
    pub overwrite_sector: bool,
}

impl WritingQueueEntry {
    pub fn new(fd: FileDescriptor) -> Self {
        Self {
            fd,
            data: [0xFFu8; 5 + SECTOR_SIZE - 256],
            data_length: 0,
            overwrite_sector: false,
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
    free_sectors:
        BlockingMutex<CriticalSectionRawMutex, RefCell<BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>>>,
    flash: Mutex<CriticalSectionRawMutex, F>,
    crc: Mutex<CriticalSectionRawMutex, C>,
    opened_files: RwLock<CriticalSectionRawMutex, Vec<Option<OpenedFile>, MAX_OPENED_FILES>, 10>,
    writing_queue: Channel<CriticalSectionRawMutex, WritingQueueEntry, WRITING_QUEUE_SIZE>,
    page_writing_queue: Channel<CriticalSectionRawMutex, PageWritingQueueEntry, WRITING_QUEUE_SIZE>,
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
        let opened_files = self.opened_files.read().await;
        for opened_file in opened_files.iter() {
            if let Some(opened_file) = opened_file {
                if opened_file.file_entry.file_id == file_id {
                    return Err(());
                }
            }
        }
        drop(opened_files);

        let mut at = self.allocation_table.write().await;
        for i in 0..at.allocation_table.file_entries.len() {
            if at.allocation_table.file_entries[i].file_id == file_id {
                at.allocation_table.file_entries.remove(i);
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

    pub async fn open_file(&self, file_id: u64) -> Option<FileDescriptor> {
        let mut opened_files = self.opened_files.write().await;

        for opened_file in opened_files.iter() {
            if let Some(opened_file) = opened_file {
                if opened_file.file_entry.file_id == file_id {
                    // already opened
                    return None;
                }
            }
        }

        // find avaliable fd
        let mut file_descriptor: Option<FileDescriptor> = None;
        for fd in 0..MAX_OPENED_FILES {
            if opened_files[fd].is_none() {
                file_descriptor = Some(fd);
            }
        }
        let file_descriptor = file_descriptor?;

        let at = self.allocation_table.read().await;
        if let Some(file_entry) = self.find_file_entry(&at.allocation_table, file_id) {
            let opened_file = OpenedFile {
                current_sector_index: file_entry.first_sector_index,
                file_entry: file_entry.clone(),
            };

            opened_files[file_descriptor].replace(opened_file);
            return Some(file_descriptor);
        }

        None
    }

    pub async fn write_file(&mut self, data: WritingQueueEntry) {
        self.writing_queue.send(data).await;
    }

    pub async fn read_file<'a>(
        &self,
        fd: FileDescriptor,
        buffer: &'a mut [u8],
    ) -> Option<&'a [u8]> {
        let mut opened_files = self.opened_files.write().await;
        if let Some(opened_file) = &mut opened_files[fd] {
            let mut bytes_read: usize = 0;
            let mut current_sector_index = opened_file.file_entry.first_sector_index;
            drop(opened_files);
            let mut flash = self.flash.lock().await;
            while let Some(sector_index) = current_sector_index {
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
                let buffer = &mut buffer[bytes_read..];
                flash.read(sector_address, 8, buffer).await;
                let data_length_in_sector = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                let data_length_in_sector_padded = (data_length_in_sector + 3) & !3;

                flash
                    .read(
                        sector_address + 8,
                        data_length_in_sector_padded as usize + 4,
                        buffer,
                    )
                    .await;
                let crc_actual = self
                    .crc
                    .lock()
                    .await
                    .calculate(&buffer[5..(data_length_in_sector_padded as usize + 5)]);
                let crc_expected = u32::from_be_bytes(
                    (&buffer[(data_length_in_sector_padded as usize + 5)
                        ..(data_length_in_sector_padded as usize + 9)])
                        .try_into()
                        .unwrap(),
                );
                if crc_actual != crc_expected {
                    warn!(
                        "CRC mismatch: expected {}, got {}",
                        crc_expected, crc_actual
                    );
                    return None;
                }

                // read next sector index
                let buffer = &mut buffer[data_length_in_sector as usize..];
                flash.read(sector_address + 4096 - 256, 8, buffer).await;
                let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };

                bytes_read += data_length_in_sector as usize;
            }

            return Some(&buffer[5..(5 + bytes_read)]);
        }

        None
    }

    // TODO optimize
    fn find_avaliable_sector(&self) -> Option<u16> {
        self.free_sectors.lock(|free_sectors|{
            let free_sectors = free_sectors.borrow();
            for i in 0..SECTORS_COUNT {
                if !free_sectors[i] {
                    return Some(i.try_into().unwrap());
                }
            }
            None
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
