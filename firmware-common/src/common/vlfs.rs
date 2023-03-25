use bitvec::prelude::*;
use heapless::Vec;

use crate::driver::flash::SpiFlash;

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

const VLFS_VERSION: u32 = 1;

pub struct VLFS<F, const MAX_FILES: usize, const TABLE_COUNT: usize, const SECTORS_COUNT: usize>
where
    F: SpiFlash,
    [u32; SECTORS_COUNT / 32]: Sized,
    [u8; 12 + MAX_FILES * 12 + 5]: Sized,
{
    version: u32,
    allocation_table: AllocationTable<MAX_FILES>,
    free_sectors: BitArray<[u32; SECTORS_COUNT / 32], Lsb0>,
    flash: F,
}

impl<F, const MAX_FILES: usize, const TABLE_COUNT: usize, const SECTORS_COUNT: usize>
    VLFS<F, MAX_FILES, TABLE_COUNT, SECTORS_COUNT>
where
    F: SpiFlash,
    [u32; SECTORS_COUNT / 32]: Sized,
    [u8; 12 + MAX_FILES * 12 + 5]: Sized,
{
    pub fn new() -> Self {
        assert!(12 + MAX_FILES * 12 < 32*1024);
        todo!()
    }

    pub async fn read_allocation_table(flash: &mut F) -> Result<(), ()> {
        let tables: Vec<AllocationTable<MAX_FILES>, TABLE_COUNT> = Vec::new();
        let table_max_size = 12 + MAX_FILES * 12;
        let mut buffer = [0u8; 12 + MAX_FILES * 12 + 5];
        for i in 0..TABLE_COUNT {
            flash.read_4kb((i*32*1024).try_into().unwrap() , table_max_size, &mut buffer);
            // let mut buffer = flash.read_256_bytes(i * 256).await;
            // let length = buffer.read_u32() as usize;
            // let table = deserialize_safe!(AllocationTable<MAX_FILES>, buffer.read_slice(length));
            // if let Some(table) = table {
            //     tables.push(table);
            // }
        }

        Ok(())
    }
}

fn a() {
    let arr = bitarr![u32, Lsb0; 0; 80];
}
