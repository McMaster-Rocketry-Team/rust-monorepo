use super::{error::VLFSError, utils::find_most_common_u16_out_of_4, *};
use crate::{
    driver::{crc::Crc, flash::Flash},
    utils::rwlock::RwLock,
};
use embassy_sync::mutex::Mutex;

use embassy_sync::blocking_mutex::Mutex as BlockingMutex;

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        Self {
            allocation_table: RwLock::new(AllocationTable::default()),
            sectors_mng: RwLock::new(SectorsMng::new()),
            flash: RwLock::new(FlashWrapper::new(flash)),
            crc: Mutex::new(crc),
            rng: BlockingMutex::new(RefCell::new(SmallRng::seed_from_u64(0))),
        }
    }

    pub async fn init(&mut self) -> Result<(), VLFSError<F::Error>> {
        if self.read_latest_allocation_table().await? {
            let at = self.allocation_table.read().await;
            log_info!(
                "Found valid allocation table, file count: {}",
                at.footer.file_count
            );
            drop(at);

            self.read_free_sectors().await?;

            let sectors_mng = self.sectors_mng.read().await;
            let total_sectors = SECTORS_COUNT - ALLOC_TABLES_SECTORS_USED;
            let used_sectors = total_sectors - sectors_mng.sector_map.free_sectors_count as usize;
            let free_space =
                (sectors_mng.sector_map.free_sectors_count as usize * MAX_SECTOR_DATA_SIZE) / 1024;
                log_info!(
                "{} out of {} sectors used, avaliable space: {}KiB",
                used_sectors, total_sectors, free_space,
            );
        } else {
            log_info!("No valid allocation table found, creating a new one");
            self.write_empty_allocation_table().await?;
        }

        let crc = self.crc.get_mut();
        let mut sectors_mng = self.sectors_mng.write().await;
        let crc = crc.calculate_u32(&sectors_mng.sector_map.map_4k.data);
        sectors_mng.rng = SmallRng::seed_from_u64(crc as u64 + ((crc as u64) << 32));

        self.rng.lock(|rng| {
            rng.replace(SmallRng::seed_from_u64(sectors_mng.rng.next_u64()));
        });

        log_info!("VLFS initialized");
        Ok(())
    }

    async fn read_free_sectors(&mut self) -> Result<(), VLFSError<F::Error>> {
        let mut iter = self.files_iter().await;
        while let Some(file_entry) = iter.next().await? {
            let mut current_sector_index = file_entry.first_sector_index;
                while let Some(sector_index) = current_sector_index {
                    log_trace!("at sector {:#X}", sector_index);
                    self.claim_sector(sector_index).await;

                    let mut buffer = [0u8; 5 + 8];
                    let next_sector_index_address =
                        (sector_index as usize * SECTOR_SIZE + SECTOR_SIZE - 8) as u32;
                    self.flash
                        .read()
                        .await
                        .read(next_sector_index_address, 8, &mut buffer)
                        .await
                        .map_err(VLFSError::FlashError)?;
                    let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
                    log_trace!("next_sector_index: {}", next_sector_index);
                    current_sector_index = if next_sector_index == 0xFFFF {
                        None
                    } else {
                        Some(next_sector_index)
                    };
                }
        }

        Ok(())
    }
}
