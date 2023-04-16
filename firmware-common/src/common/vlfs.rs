use crate::common::buffer::WriteBuffer;
use crate::driver::flash::{IOReader, SpiReader};
use crate::driver::{crc::Crc, flash::SpiFlash};
use crate::new_write_buffer;
use bitvec::prelude::*;
use defmt::*;
use heapless::Vec;

const SECTORS_COUNT: usize = 65536;
const MAX_FILES: usize = 256; // can be as large as 2728, however that breaks the 256kb stack for some reason
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = 2048; // SECTORS_COUNT / 32
const ALLOC_TABLE_READ_LENGTH: usize = 3088; // 16 + MAX_FILES * 12

pub struct FileEntry {
    pub metadata: [u8; 10],
    pub first_sector_index: u16,
}

// serialized size must fit in half a block (32kib)
pub struct AllocationTable {
    pub sequence_number: u32,
    pub file_count: u32,
    pub file_entries: Vec<FileEntry, MAX_FILES>,
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

const VLFS_VERSION: u32 = 1;

pub struct VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub version: u32,
    pub allocation_table: AllocationTable,
    pub allocation_table_index: usize, // which half block is the allocation table in
    pub free_sectors: BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>,
    pub flash: F,
    pub crc: C,
}

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        Self {
            version: VLFS_VERSION,
            allocation_table: AllocationTable::default(),
            allocation_table_index: TABLE_COUNT - 1,
            free_sectors: BitArray::<_, Lsb0>::new([0u32; FREE_SECTORS_ARRAY_SIZE]),
            flash,
            crc,
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

    async fn read_allocation_table(&mut self) -> bool {
        let mut found_valid_table = false;

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i);

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
                let metadata = reader.read_slice(10).await;
                let metadata: [u8; 10] = metadata.try_into().unwrap();
                let first_sector_index = reader.read_u16().await;
                files
                    .push(FileEntry {
                        metadata,
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
        new_write_buffer!(write_buffer, ALLOC_TABLE_READ_LENGTH);

        write_buffer.extend_from_u32(VLFS_VERSION);
        write_buffer.extend_from_u32(self.allocation_table.sequence_number);
        write_buffer.extend_from_u32(self.allocation_table.file_count);
        for file in &self.allocation_table.file_entries {
            write_buffer.extend_from_slice(&file.metadata);
            write_buffer.extend_from_u16(file.first_sector_index);
        }

        self.crc.reset();
        for i in 0..(write_buffer.len() / 4) {
            info!(
                "feed word: {=[?]}",
                write_buffer.as_slice_without_start()[(i * 4)..((i + 1) * 4)]
            );
            self.crc.feed(u32::from_be_bytes(
                write_buffer.as_slice_without_start()[(i * 4)..((i + 1) * 4)]
                    .try_into()
                    .unwrap(),
            ));
        }
        let crc = self.crc.read();
        info!("CRC: {}", crc);
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
