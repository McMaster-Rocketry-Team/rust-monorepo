use super::*;

pub(super) struct SectorsMng {
    pub(super) sector_map: BitArray<[u32; SECTOR_MAP_ARRAY_SIZE], Lsb0>, // false: unused; true: used
    pub(super) free_sectors_count: u32,
    pub(super) rng: SmallRng,
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub(super) async fn claim_avaliable_sector_and_erase(&self) -> Result<u16, VLFSError<F>> {
        let result = self.sectors_mng.lock(|sectors_mng| {
            let mut sectors_mng = sectors_mng.borrow_mut();
            if sectors_mng.free_sectors_count == 0 {
                return Err(VLFSError::DeviceFull);
            }

            let start_i = (sectors_mng.rng.next_u64() as usize) % DATA_REGION_SECTORS;
            for i in start_i..DATA_REGION_SECTORS {
                if !sectors_mng.sector_map[i] {
                    sectors_mng.sector_map.set(i, true);
                    sectors_mng.free_sectors_count -= 1;
                    return Ok((i + ALLOC_TABLES_SECTORS_USED) as u16);
                }
            }
            for i in 0..start_i {
                if !sectors_mng.sector_map[i] {
                    sectors_mng.sector_map.set(i, true);
                    sectors_mng.free_sectors_count -= 1;
                    return Ok((i + ALLOC_TABLES_SECTORS_USED) as u16);
                }
            }

            defmt::panic!("wtf");
        });

        if let Ok(i) = result {
            info!("Claimed sector #{:#X}", i);
            self.flash
                .lock()
                .await
                .erase_sector_4kib(i as u32 * SECTOR_SIZE as u32)
                .await
                .map_err(VLFSError::from_flash)?;
        }

        result
    }

    pub(super) fn claim_sector(&self, sector_index: u16) {
        self.sectors_mng.lock(|sectors_mng| {
            let mut sectors_mng = sectors_mng.borrow_mut();
            let offsetted_sector_index = sector_index - ALLOC_TABLES_SECTORS_USED as u16;
            if !sectors_mng.sector_map[offsetted_sector_index as usize] {
                sectors_mng.free_sectors_count -= 1;
                sectors_mng
                    .sector_map
                    .set(offsetted_sector_index as usize, true);
            }
        });
    }

    pub(super) fn return_sector(&self, sector_index: u16) {
        self.sectors_mng.lock(|sectors_mng| {
            let mut sectors_mng = sectors_mng.borrow_mut();
            let offsetted_sector_index = sector_index - ALLOC_TABLES_SECTORS_USED as u16;
            if sectors_mng.sector_map[offsetted_sector_index as usize] {
                sectors_mng.free_sectors_count += 1;
                sectors_mng
                    .sector_map
                    .set(offsetted_sector_index as usize, false);
            }
        });
    }
}
