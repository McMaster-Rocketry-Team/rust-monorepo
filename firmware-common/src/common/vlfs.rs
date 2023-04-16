use crate::common::buffer::WriteBuffer;
use crate::driver::flash::{IOReader, SpiReader};
use crate::driver::{crc::Crc, flash::SpiFlash};
use crate::new_write_buffer;
use bitvec::prelude::*;
use defmt::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use heapless::Vec;

const VLFS_VERSION: u32 = 1;
const SECTORS_COUNT: usize = 65536;
const MAX_FILES: usize = 512; // can be as large as 2728
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = 2048; // SECTORS_COUNT / 32
const MAX_ALLOC_TABLE_LENGTH: usize = 3088; // 16 + MAX_FILES * 12
const MAX_OPENED_FILES: usize = 10;
const WRITING_QUEUE_SIZE: usize = 4;

#[derive(Debug, Clone)]
struct FileEntry {
    file_id: u64,
    file_type: u16,
    first_sector_index: u16,
}

// serialized size must fit in half a block (32kib)
struct AllocationTable {
    sequence_number: u32,
    file_count: u32,
    file_entries: Vec<FileEntry, MAX_FILES>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            file_count: 0,
            file_entries: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct OpenedFile {
    current_sector_index: u16,
    file_entry: FileEntry, // FIXME use reference
}

type FileDescriptor = usize;

pub struct WritingQueueEntry {
    pub fd: FileDescriptor,
    pub data: [u8; 5 + 4096],
}

pub struct VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    allocation_table: AllocationTable,
    allocation_table_index: usize, // which half block is the allocation table in
    free_sectors: BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>,
    flash: F,
    crc: C,
    opened_files: Vec<Option<OpenedFile>, MAX_OPENED_FILES>,
    writing_queue: Channel<CriticalSectionRawMutex, WritingQueueEntry, WRITING_QUEUE_SIZE>,
}

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        let mut opened_files = Vec::<Option<OpenedFile>, MAX_OPENED_FILES>::new();
        for _ in 0..MAX_OPENED_FILES {
            opened_files.push(None).unwrap();
        }
        Self {
            allocation_table: AllocationTable::default(),
            allocation_table_index: TABLE_COUNT - 1,
            free_sectors: BitArray::<_, Lsb0>::new([0u32; FREE_SECTORS_ARRAY_SIZE]),
            flash,
            crc,
            opened_files: Vec::new(),
            writing_queue: Channel::new(),
        }
    }

    pub async fn init(&mut self) {
        if self.read_allocation_table().await {
            info!(
                "Found valid allocation table, file count: {}",
                self.allocation_table.file_count
            );
        } else {
            info!("No valid allocation table found, creating new one");
            self.write_allocation_table().await;
        }
        info!("VLFS initialized");
    }

    pub async fn open(&mut self, file_id: u64) -> Option<FileDescriptor> {
        let file_descriptor = self.next_avaliable_file_descriptor()?;
        for file_entry in &self.allocation_table.file_entries {
            if file_entry.file_id == file_id {
                let opened_file = OpenedFile {
                    current_sector_index: file_entry.first_sector_index,
                    file_entry: file_entry.clone(),
                };
                self.opened_files[file_descriptor].replace(opened_file);
                return Some(file_descriptor);
            }
        }

        None
    }

    pub async fn write_file(&mut self, data: WritingQueueEntry){
        self.writing_queue.send(data).await;
    }

    pub async fn run(&self){

    }

    fn next_avaliable_file_descriptor(&self) -> Option<FileDescriptor> {
        for fd in 0..MAX_OPENED_FILES {
            if self.opened_files[fd].is_none() {
                return Some(fd);
            }
        }
        None
    }

    async fn read_allocation_table(&mut self) -> bool {
        let mut found_valid_table = false;

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i + 1);

            let mut reader = SpiReader::new(
                (i * 32 * 1024).try_into().unwrap(),
                &mut self.flash,
                &mut self.crc,
            );

            reader.reset_crc();
            let version = reader.read_u32().await;
            if version != VLFS_VERSION {
                warn!("Version mismatch for allocation table #{}", i);
                continue;
            }

            let sequence_number = reader.read_u32().await;
            let file_count = reader.read_u32().await;
            if file_count > MAX_FILES.try_into().unwrap() {
                warn!("file_count > MAX_FILES");
                continue;
            }
            let mut files: Vec<FileEntry, MAX_FILES> = Vec::<FileEntry, MAX_FILES>::new();
            for _ in 0..file_count {
                let file_id = reader.read_u64().await;
                let file_type = reader.read_u16().await;
                let first_sector_index = reader.read_u16().await;
                files
                    .push(FileEntry {
                        file_id,
                        file_type,
                        first_sector_index,
                    })
                    .ok()
                    .unwrap();
            }

            let actual_crc = reader.get_crc();
            let expected_crc = reader.read_u32().await;
            if actual_crc == expected_crc {
                info!("CRC match!");
            } else {
                warn!(
                    "CRC mismatch! expected: {}, actual: {}",
                    expected_crc, actual_crc
                );
                continue;
            }

            if sequence_number > self.allocation_table.sequence_number {
                found_valid_table = true;
                self.allocation_table = AllocationTable {
                    sequence_number,
                    file_count,
                    file_entries: files,
                };
            }
        }

        found_valid_table
    }

    async fn write_allocation_table(&mut self) {
        self.allocation_table_index = (self.allocation_table_index + 1) % TABLE_COUNT;
        self.allocation_table.sequence_number += 1;
        new_write_buffer!(write_buffer, MAX_ALLOC_TABLE_LENGTH);

        write_buffer.extend_from_u32(VLFS_VERSION);
        write_buffer.extend_from_u32(self.allocation_table.sequence_number);
        write_buffer.extend_from_u32(self.allocation_table.file_count);
        for file in &self.allocation_table.file_entries {
            write_buffer.extend_from_u64(file.file_id);
            write_buffer.extend_from_u16(file.file_type);
            write_buffer.extend_from_u16(file.first_sector_index);
        }

        let crc = write_buffer.calculate_crc(&mut self.crc);
        write_buffer.extend_from_u32(crc);

        info!(
            "write_buffer: {=[?]}",
            write_buffer.as_slice_without_start()
        );

        self.flash
            .erase_block_32kib((self.allocation_table_index * 32 * 1024) as u32)
            .await;
        self.flash
            .write(
                (self.allocation_table_index * 32 * 1024) as u32,
                write_buffer.len(),
                &mut write_buffer.as_mut_slice(),
            )
            .await;
    }
}

pub async fn test_flash<F: SpiFlash, C: Crc>(flash: &mut F, crc: &mut C) {
    {
        info!("erasing block");
        flash.erase_block_64kib(0).await;
        info!("erased");

        let mut write_buffer = [0u8; 40965];
        info!("buffer created");
        for i in 0..40960 {
            write_buffer[i + 5] = (i % 33) as u8;
        }

        info!("writing");

        flash.write(0, 40960, &mut write_buffer).await;

        info!("done writing");
    }

    {
        let mut reader = SpiReader::new(0, flash, crc);
        for i in 0..40960 {
            let b = reader.read_u8().await;
            if b != (i % 33) as u8 {
                error!(
                    "read error at index: {}, expected: {}, read: {}",
                    i,
                    (i % 33) as u8,
                    b
                );
                loop {}
            }
        }
    }

    info!("success");
}
