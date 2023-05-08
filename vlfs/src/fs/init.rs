use super::{error::VLFSError, utils::find_most_common_u16_out_of_4, *};
use crate::{
    driver::{crc::Crc, flash::Flash},
    utils::flash_io::{FlashReader, FlashWriter},
    utils::io_traits::{AsyncReader, AsyncWriter},
    utils::rwlock::RwLock,
};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use heapless::Vec;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        let mut sector_map = BitArray::<_, Lsb0>::new([0u32; SECTOR_MAP_ARRAY_SIZE]);
        for i in 0..ALLOC_TABLES_SECTORS_USED {
            sector_map.set(i, true);
        }
        Self {
            allocation_table: RwLock::new(AllocationTableWrapper::default()),
            free_sectors: BlockingMutex::new(RefCell::new(FreeSectors {
                phantom: PhantomData,
                sector_map,
                free_sectors_count: (SECTORS_COUNT - ALLOC_TABLES_SECTORS_USED) as u32,
                rng: SmallRng::seed_from_u64(
                    0b1010011001010000010000000111001110111101011110001100000011100000u64,
                ),
            })),
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
        }
    }

    pub async fn init(&mut self) -> Result<(), VLFSError<F>> {
        if self.read_allocation_table().await? {
            let at = self.allocation_table.read().await;
            info!(
                "Found valid allocation table, file count: {}",
                at.allocation_table.file_entries.len()
            );
            drop(at);
            self.read_free_sectors().await?;
            self.free_sectors.lock(|free_sectors| {
                let free_sectors = free_sectors.borrow();

                let total_sectors = SECTORS_COUNT - ALLOC_TABLES_SECTORS_USED;
                let used_sectors = total_sectors - free_sectors.free_sectors_count as usize;
                let free_space =
                    (free_sectors.free_sectors_count as usize * MAX_SECTOR_DATA_SIZE) / 1024;
                info!(
                    "{} out of {} sectors used, avaliable space: {}KiB",
                    used_sectors, total_sectors, free_space,
                );
            });
        } else {
            info!("No valid allocation table found, creating a new one");
            self.write_allocation_table().await?;
        }
        info!("VLFS initialized");
        Ok(())
    }

    async fn read_free_sectors(&mut self) -> Result<(), VLFSError<F>> {
        let at = self.allocation_table.read().await;
        for file_entry in &at.allocation_table.file_entries {
            let mut current_sector_index = file_entry.first_sector_index;
            while let Some(sector_index) = current_sector_index {
                trace!("at sector {:#X}", sector_index);
                self.free_sectors.lock(|free_sectors| {
                    let mut free_sectors = free_sectors.borrow_mut();
                    free_sectors.claim_sector(sector_index);
                });

                let mut buffer = [0u8; 5 + 8];
                let next_sector_index_address =
                    (sector_index as usize * SECTOR_SIZE + SECTOR_SIZE - 8) as u32;
                self.flash
                    .get_mut()
                    .read(next_sector_index_address, 8, &mut buffer)
                    .await
                    .map_err(VLFSError::from_flash)?;
                let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                trace!("next_sector_index: {}", next_sector_index);
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
            }
        }

        Ok(())
    }

    async fn read_allocation_table(&mut self) -> Result<bool, VLFSError<F>> {
        let mut found_valid_table = false;
        let mut at = self.allocation_table.write().await;

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i + 1);

            let flash = self.flash.get_mut();
            let crc = self.crc.get_mut();
            let mut read_buffer = [0u8; 5 + 12];
            let mut reader = FlashReader::new((i * 32 * 1024).try_into().unwrap(), flash, crc);

            let read_result = reader
                .read_slice(&mut read_buffer, 12)
                .await
                .map_err(VLFSError::from_flash)?;
            let version = u32::from_be_bytes((&read_result[0..4]).try_into().unwrap());
            let sequence_number = u32::from_be_bytes((&read_result[4..8]).try_into().unwrap());
            let file_count = u32::from_be_bytes((&read_result[8..12]).try_into().unwrap());
            if version != VLFS_VERSION {
                warn!(
                    "Version mismatch, expected: {}, actual: {}",
                    VLFS_VERSION, version
                );
                continue;
            }
            if file_count > MAX_FILES as u32 {
                warn!("file_count > MAX_FILES");
                continue;
            }
            let mut files: Vec<FileEntry, MAX_FILES> = Vec::<FileEntry, MAX_FILES>::new();
            for _ in 0..file_count {
                let read_result = reader
                    .read_slice(&mut read_buffer, 12)
                    .await
                    .map_err(VLFSError::from_flash)?;
                let file_id = u64::from_be_bytes((&read_result[0..8]).try_into().unwrap());
                let file_type = u16::from_be_bytes((&read_result[8..10]).try_into().unwrap());
                let first_sector_index =
                    u16::from_be_bytes((&read_result[10..12]).try_into().unwrap());
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
                    .unwrap();
            }

            let actual_crc = reader.get_crc();
            let expected_crc = reader
                .read_u32(&mut read_buffer)
                .await
                .map_err(VLFSError::from_flash)?;
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

        Ok(found_valid_table)
    }

    pub(super) async fn write_allocation_table(&self) -> Result<(), VLFSError<F>> {
        let mut at = self.allocation_table.write().await;
        at.allocation_table_index = (at.allocation_table_index + 1) % TABLE_COUNT;
        at.allocation_table.sequence_number += 1;
        drop(at);

        let at = self.allocation_table.read().await;
        let at_address = (at.allocation_table_index * 32 * 1024) as u32;

        let mut flash = self.flash.lock().await;
        flash
            .erase_block_32kib(at_address)
            .await
            .map_err(VLFSError::from_flash)?;

        let mut crc = self.crc.lock().await;
        let mut writer = FlashWriter::new(
            at_address,
            &mut flash,
            &mut crc,
        );

        writer
            .extend_from_u32(VLFS_VERSION)
            .await
            .map_err(VLFSError::from_flash)?;
        writer
            .extend_from_u32(at.allocation_table.sequence_number)
            .await
            .map_err(VLFSError::from_flash)?;
        writer
            .extend_from_u32(at.allocation_table.file_entries.len() as u32)
            .await
            .map_err(VLFSError::from_flash)?;

        for file in &at.allocation_table.file_entries {
            writer
                .extend_from_u64(file.file_id)
                .await
                .map_err(VLFSError::from_flash)?;
            writer
                .extend_from_u16(file.file_type)
                .await
                .map_err(VLFSError::from_flash)?;
            if let Some(first_sector_index) = file.first_sector_index {
                writer
                    .extend_from_u16(first_sector_index)
                    .await
                    .map_err(VLFSError::from_flash)?;
            } else {
                writer
                    .extend_from_u16(0xFFFF)
                    .await
                    .map_err(VLFSError::from_flash)?;
            }
        }

        info!("CRC: {}", writer.get_crc());
        writer
            .extend_from_u32(writer.get_crc())
            .await
            .map_err(VLFSError::from_flash)?;
        writer.flush().await.map_err(VLFSError::from_flash)?;

        Ok(())
    }
}
