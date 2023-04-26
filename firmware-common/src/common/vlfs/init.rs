use super::{utils::find_most_common_u16_out_of_4, *};
use crate::{
    common::{
        io_traits::{AsyncReader, Writer},
        rwlock::RwLock,
    },
    driver::{
        crc::Crc,
        flash::{SpiFlash, SpiReader},
    },
};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use heapless::Vec;

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        let mut free_sectors = BitArray::<_, Lsb0>::new([0u32; FREE_SECTORS_ARRAY_SIZE]);
        for i in 0..(TABLE_COUNT * 32 * 1024 / SECTOR_SIZE) {
            free_sectors.set(i, true);
        }
        Self {
            allocation_table: RwLock::new(AllocationTableWrapper::default()),
            free_sectors: BlockingMutex::new(RefCell::new(free_sectors)),
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
            writing_queue: Channel::new(),
        }
    }

    pub async fn init(&mut self) {
        if self.read_allocation_table().await {
            let at = self.allocation_table.read().await;
            info!(
                "Found valid allocation table, file count: {}",
                at.allocation_table.file_entries.len()
            );
            drop(at);
            let used_sectors = self.read_free_sectors().await;
            info!(
                "{} out of {} sectors used",
                used_sectors,
                self.flash.get_mut().size() / SECTOR_SIZE as u32
                    - (TABLE_COUNT * 32 * 1024 / SECTOR_SIZE) as u32
            );
        } else {
            info!("No valid allocation table found, creating new one");
            self.write_allocation_table().await;
        }
        info!("VLFS initialized");
    }

    // returns amount of used sectors
    async fn read_free_sectors(&mut self) -> usize {
        let mut used_sectors: usize = 0;

        let at = self.allocation_table.read().await;
        for file_entry in &at.allocation_table.file_entries {
            let mut current_sector_index = file_entry.first_sector_index;
            while let Some(sector_index) = current_sector_index {
                self.free_sectors.lock(|free_sectors| {
                    let mut free_sectors = free_sectors.borrow_mut();
                    let free_sectors = free_sectors.as_mut_bitslice();
                    free_sectors.set(sector_index as usize, true);
                });

                used_sectors += 1;

                let mut buffer = [0u8; 5 + 8];
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
                self.flash
                    .get_mut()
                    .read(sector_address + 4096 - 256, 8, &mut buffer)
                    .await;
                let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
            }
        }

        used_sectors
    }

    async fn read_allocation_table(&mut self) -> bool {
        let mut found_valid_table = false;
        let mut at = self.allocation_table.write().await;

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i + 1);

            let flash = self.flash.get_mut();
            let crc = self.crc.get_mut();
            let mut read_buffer = [0u8; 4 + 5];
            let mut reader = SpiReader::new((i * 32 * 1024).try_into().unwrap(), flash, crc);

            reader.reset_crc();
            let version = reader.read_u32(&mut read_buffer).await;
            if version != VLFS_VERSION {
                warn!(
                    "Version mismatch for allocation table #{}, expected: {}, actual: {}",
                    i, VLFS_VERSION, version
                );
                continue;
            }

            let sequence_number = reader.read_u32(&mut read_buffer).await;
            let file_count = reader.read_u32(&mut read_buffer).await;
            if file_count > MAX_FILES.try_into().unwrap() {
                warn!("file_count > MAX_FILES");
                continue;
            }
            let mut files: Vec<FileEntry, MAX_FILES> = Vec::<FileEntry, MAX_FILES>::new();
            for _ in 0..file_count {
                let file_id = reader.read_u64(&mut read_buffer).await;
                let file_type = reader.read_u16(&mut read_buffer).await;
                let first_sector_index = reader.read_u16(&mut read_buffer).await;
                files
                    .push(FileEntry {
                        file_id,
                        file_type,
                        first_sector_index: if first_sector_index == 0xFFFF {
                            None
                        } else {
                            Some(first_sector_index)
                        },
                        opened: false,
                    })
                    .ok()
                    .unwrap();
            }

            let actual_crc = reader.get_crc();
            let expected_crc = reader.read_u32(&mut read_buffer).await;
            if actual_crc == expected_crc {
                info!("CRC match!");
            } else {
                warn!(
                    "CRC mismatch! expected: {}, actual: {}",
                    expected_crc, actual_crc
                );
                continue;
            }

            if sequence_number > at.allocation_table.sequence_number {
                found_valid_table = true;
                at.allocation_table = AllocationTable {
                    sequence_number,
                    file_entries: files,
                };
                at.allocation_table_index = i;
            }
        }

        found_valid_table
    }

    pub(super) async fn write_allocation_table(&self) {
        let mut at = self.allocation_table.write().await;
        at.allocation_table_index = (at.allocation_table_index + 1) % TABLE_COUNT;
        at.allocation_table.sequence_number += 1;
        drop(at);

        let at = self.allocation_table.read().await;

        new_write_buffer!(write_buffer, MAX_ALLOC_TABLE_LENGTH);

        write_buffer.extend_from_u32(VLFS_VERSION);
        write_buffer.extend_from_u32(at.allocation_table.sequence_number);
        write_buffer.extend_from_u32(at.allocation_table.file_entries.len() as u32);
        for file in &at.allocation_table.file_entries {
            write_buffer.extend_from_u64(file.file_id);
            write_buffer.extend_from_u16(file.file_type);
            if let Some(first_sector_index) = file.first_sector_index {
                write_buffer.extend_from_u16(first_sector_index);
            } else {
                write_buffer.extend_from_u16(0xFFFF);
            }
        }

        let mut crc = self.crc.lock().await;
        let crc = crc.calculate(write_buffer.as_slice_without_start());
        write_buffer.extend_from_u32(crc);

        info!(
            "write_buffer: {=[?]}",
            write_buffer.as_slice_without_start()
        );

        let mut flash = self.flash.lock().await;
        flash
            .erase_block_32kib((at.allocation_table_index * 32 * 1024) as u32)
            .await;
        flash
            .write(
                (at.allocation_table_index * 32 * 1024) as u32,
                write_buffer.len(),
                &mut write_buffer.as_mut_slice(),
            )
            .await;
    }
}
