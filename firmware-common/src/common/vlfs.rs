use crate::driver::flash::WriteBuffer;
use crate::{
    driver::{
        crc::Crc,
        flash::{ReadBuffer, SpiFlash},
    },
    new_read_buffer, new_write_buffer,
};
use bitvec::prelude::*;
use defmt::*;
use heapless::Vec;

const SECTORS_COUNT: usize = 65536;
const MAX_FILES: usize = 2728;
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = 2048; // SECTORS_COUNT / 32

const ALLOC_TABLE_READ_LENGTH: usize = 32752; // 16 + MAX_FILES * 12
                                              // const ALLOC_TABLE_READ_BUFFER_LENGTH: usize = 32757; // 16 + MAX_FILES * 12 + 5

struct FileEntry {
    pub metadata: [u8; 10],
    pub first_sector_index: u16,
}

// serialized size must fit in half a block
struct AllocationTable {
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
    version: u32,
    allocation_table: AllocationTable,
    allocation_table_index: usize, // which half block is the allocation table in
    free_sectors: BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>,
    flash: F,
    crc: C,
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
        info!("Initializing VLFS inner");
        info!(
            "{} {}",
            self.allocation_table.file_entries.len(),
            self.free_sectors.len()
        );
        self.read_allocation_table().await;
        // if self.read_allocation_table().await {
        //     info!(
        //         "Found valid allocation table, file count: {}",
        //         self.allocation_table.file_count
        //     );
        //     // self.allocation_table = table;
        // } else {
        //     info!("No valid allocation table found, creating new one");
        //     // self.write_allocation_table().await;
        // }
    }

    async fn read_allocation_table(&mut self) -> bool {
        let mut last_table: Option<AllocationTable> = None;

        new_read_buffer!(read_buffer, ALLOC_TABLE_READ_LENGTH);
        info!(
            "read_buffer: {}, {}",
            read_buffer.len(),
            &read_buffer.as_mut_slice()[0..10]
        );

        info!("Searching for valid allocation table...");

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i);
            read_buffer.reset();
            let a: u32 = (i * 32 * 1024).try_into().unwrap();
            info!("{} {}", a, ALLOC_TABLE_READ_LENGTH);
            let b = ALLOC_TABLE_READ_LENGTH;
            self.flash
                .read(
                    a,
                    b,
                    &mut read_buffer,
                )
                .await;
            info!(
                "read_buffer: {}, {}",
                read_buffer.len(),
                &read_buffer.as_mut_slice()[0..10]
            );

            //     let expected_crc = read_buffer.read_u32();
            //     let actual_crc = self
            //         .crc
            //         .calculate(&read_buffer.as_slice_without_start()[4..]);
            //     if expected_crc != actual_crc {
            //         warn!("CRC mismatch for allocation table #{}", i);
            //         continue;
            //     }

            //     let version = read_buffer.read_u32();
            //     if version != VLFS_VERSION {
            //         warn!("Version mismatch for allocation table #{}", i);
            //         continue;
            //     }

            //     let sequence_number = read_buffer.read_u32();
            //     let file_count = read_buffer.read_u32();
            //     let mut files: Vec<FileEntry, MAX_FILES> = Vec::<FileEntry, MAX_FILES>::new();
            //     for _ in 0..file_count {
            //         let metadata = read_buffer.read_slice(10);
            //         let metadata: [u8; 10] = metadata.try_into().unwrap();
            //         let first_sector_index = read_buffer.read_u16();
            //         files
            //             .push(FileEntry {
            //                 metadata,
            //                 first_sector_index,
            //             })
            //             .ok()
            //             .unwrap();
            //     }

            //     if let Some(table) = last_table.take() {
            //         if sequence_number > table.sequence_number {
            //             last_table.replace(AllocationTable {
            //                 sequence_number,
            //                 file_count,
            //                 file_entries: files,
            //             });
            //         }
            //     } else {
            //         last_table = Some(AllocationTable {
            //             sequence_number,
            //             file_count,
            //             file_entries: files,
            //         });
            //     }
        }

        true
    }

    async fn write_allocation_table(&mut self) {
        self.allocation_table_index = (self.allocation_table_index + 1) % TABLE_COUNT;
        self.allocation_table.sequence_number += 1;
        new_write_buffer!(write_buffer, ALLOC_TABLE_READ_LENGTH);

        write_buffer.extend_from_u32(0); // checksum
        write_buffer.extend_from_u32(VLFS_VERSION);
        write_buffer.extend_from_u32(self.allocation_table.sequence_number);
        write_buffer.extend_from_u32(self.allocation_table.file_count);
        for file in &self.allocation_table.file_entries {
            write_buffer.extend_from_slice(&file.metadata);
            write_buffer.extend_from_u16(file.first_sector_index);
        }

        let crc = self
            .crc
            .calculate(&write_buffer.as_slice_without_start()[4..]);
        info!("CRC: {}", crc);
        write_buffer.replace_u32(crc, 0);

        self.flash
            .erase_block_32kib((self.allocation_table_index * 32 * 1024) as u32)
            .await;
        self.flash
            .write(
                (self.allocation_table_index * 32 * 1024) as u32,
                write_buffer.len(),
                &mut write_buffer,
            )
            .await;
    }
}
