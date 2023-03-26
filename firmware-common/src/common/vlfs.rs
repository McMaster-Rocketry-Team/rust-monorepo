use bitvec::prelude::*;
use defmt::*;
use heapless::Vec;

use crate::{
    driver::{
        crc::Crc,
        flash::{ReadBuffer, SpiFlash},
    },
    new_read_buffer,
};

struct FileEntry {
    pub metadata: [u8; 10],
    pub first_sector_index: u16,
}

// serialized size must fit in half a block
struct AllocationTable<const MAX_FILES: usize> {
    pub sequence_number: u32,
    pub file_count: u32,
    pub file_entries: Vec<FileEntry, MAX_FILES>,
}

impl Default for AllocationTable<0> {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            file_count: 0,
            file_entries: Vec::new(),
        }
    }
}

const VLFS_VERSION: u32 = 1;

pub struct VLFS<F, C, const MAX_FILES: usize, const TABLE_COUNT: usize, const SECTORS_COUNT: usize>
where
    F: SpiFlash,
    C: Crc,
    [u32; SECTORS_COUNT / 32]: Sized,
    [u8; 16 + MAX_FILES * 12 + 5]: Sized,
{
    version: u32,
    allocation_table: AllocationTable<MAX_FILES>,
    allocation_table_index: usize, // which half block is the allocation table in
    free_sectors: BitArray<[u32; SECTORS_COUNT / 32], Lsb0>,
    flash: F,
    crc: C,
}

impl<F, C, const MAX_FILES: usize, const TABLE_COUNT: usize, const SECTORS_COUNT: usize>
    VLFS<F, C, MAX_FILES, TABLE_COUNT, SECTORS_COUNT>
where
    F: SpiFlash,
    C: Crc,
    [u32; SECTORS_COUNT / 32]: Sized,
    [u8; 16 + MAX_FILES * 12 + 5]: Sized,
{
    pub fn new() -> Self {
        defmt::assert!(16 + MAX_FILES * 12 < 32 * 1024);
        defmt::todo!()
    }

    async fn read_allocation_table(&mut self) -> Option<AllocationTable<MAX_FILES>> {
        let mut table: Option<AllocationTable<MAX_FILES>> = None;
        new_read_buffer!(read_buffer, 16 + MAX_FILES * 12);

        for i in 0..TABLE_COUNT {
            read_buffer.reset();
            self.flash.read(
                (i * 32 * 1024).try_into().unwrap(),
                read_buffer.len(),
                &mut read_buffer,
            ).await;

            let expected_crc = read_buffer.read_u32();
            let actual_crc = self.crc.calculate(&read_buffer.as_slice_without_start()[4..]);
            if expected_crc != actual_crc {
                warn!("CRC mismatch for allocation table #{}", i);
                continue;
            }

            let version = read_buffer.read_u32();
            if version != VLFS_VERSION {
                warn!("Version mismatch for allocation table #{}", i);
                continue;
            }

            let sequence_number = read_buffer.read_u32();
            let file_count = read_buffer.read_u32();
            let mut files: Vec<FileEntry, MAX_FILES> = Vec::new();
            for _ in 0..file_count {
                let metadata = read_buffer.read_slice(10);
                let metadata: [u8; 10] = metadata.try_into().unwrap();
                let first_sector_index = read_buffer.read_u16();
                files
                    .push(FileEntry {
                        metadata,
                        first_sector_index,
                    })
                    .ok()
                    .unwrap();
            }

            if let Some(last_table) = table.take() {
                if sequence_number > last_table.sequence_number {
                    table.replace(AllocationTable {
                        sequence_number,
                        file_count,
                        file_entries: files,
                    });
                } else {
                    table.replace(last_table);
                }
            } else {
                table.replace(AllocationTable {
                    sequence_number,
                    file_count,
                    file_entries: files,
                });
            }
        }

        table
    }

    async fn write_allocation_table(&mut self) {
        self.allocation_table_index = (self.allocation_table_index + 1) % TABLE_COUNT;
        
    }
}

fn a() {
    let arr = bitarr![u32, Lsb0; 0; 80];
}
