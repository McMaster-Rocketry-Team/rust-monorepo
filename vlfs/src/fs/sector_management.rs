use super::*;

pub(super) struct SectorsMng {
    pub(super) sector_map: BitArray<[u32; SECTOR_MAP_ARRAY_SIZE], Lsb0>, // false: unused; true: used
    pub(super) erase_ahead_sectors: Vec<u16, 16>,
    pub(super) free_sectors_count: u32,
    pub(super) rng: SmallRng,
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub(super) async fn claim_avaliable_sector_and_erase(&self) -> Result<u16, VLFSError<F>> {
        let mut sectors_mng = self.sectors_mng.write().await;

        if sectors_mng.free_sectors_count == 0 {
            return Err(VLFSError::DeviceFull);
        }

        if let Some(i) = sectors_mng.erase_ahead_sectors.pop() {
            sectors_mng.sector_map.set(i as usize, true);
            sectors_mng.free_sectors_count -= 1;
            return Ok((i as usize + ALLOC_TABLES_SECTORS_USED) as u16);
        }

        let start_sector_i = (sectors_mng.rng.next_u64() as usize) % DATA_REGION_SECTORS;

        {
            // see if it can do 64KiB erase
            let start_sector_i = start_sector_i & !0b1111;
            for i in 0..32 {
                let start_sector_i = (start_sector_i + i * 16) % DATA_REGION_SECTORS;

                let mut all_free = true;
                for j in 0..16 {
                    if sectors_mng.sector_map[start_sector_i + j] {
                        all_free = false;
                        break;
                    }
                }

                if all_free {
                    let start_sector_i_unoffseted = start_sector_i + ALLOC_TABLES_SECTORS_USED;
                    info!("Erasing 64KiB block at sector #{:X}", start_sector_i_unoffseted);
                    self.flash
                        .lock()
                        .await
                        .erase_block_64kib((start_sector_i_unoffseted * SECTOR_SIZE) as u32)
                        .await
                        .map_err(VLFSError::from_flash)?;
                    for j in 1..16 {
                        sectors_mng
                            .erase_ahead_sectors
                            .push((start_sector_i + j) as u16)
                            .unwrap();
                    }
                    sectors_mng.sector_map.set(start_sector_i as usize, true);
                    sectors_mng.free_sectors_count -= 1;
                    return Ok(start_sector_i_unoffseted as u16);
                }
            }
        }

        {
            // see if it can do 32KiB erase
            let start_sector_i = start_sector_i & !0b111;
            for i in 0..64 {
                let start_sector_i = (start_sector_i + i * 8) % DATA_REGION_SECTORS;

                let mut all_free = true;
                for j in 0..8 {
                    if sectors_mng.sector_map[start_sector_i + j] {
                        all_free = false;
                        break;
                    }
                }

                if all_free {
                    let start_sector_i_unoffseted = start_sector_i + ALLOC_TABLES_SECTORS_USED;
                    info!("Erasing 32KiB block at sector #{:X}", start_sector_i_unoffseted);
                    self.flash
                        .lock()
                        .await
                        .erase_block_32kib((start_sector_i_unoffseted * SECTOR_SIZE) as u32)
                        .await
                        .map_err(VLFSError::from_flash)?;
                    for j in 1..8 {
                        sectors_mng
                            .erase_ahead_sectors
                            .push((start_sector_i + j) as u16)
                            .unwrap();
                    }
                    sectors_mng.sector_map.set(start_sector_i as usize, true);
                    sectors_mng.free_sectors_count -= 1;
                    return Ok(start_sector_i_unoffseted as u16);
                }
            }
        }

        {
            // fallback to 4KiB erase
            for i in 0..DATA_REGION_SECTORS {
                let start_sector_i = (start_sector_i + i) % DATA_REGION_SECTORS;

                if !sectors_mng.sector_map[start_sector_i] {
                    let start_sector_i_unoffseted = start_sector_i + ALLOC_TABLES_SECTORS_USED;
                    info!("Erasing 4KiB at sector #{:X}", start_sector_i_unoffseted);
                    self.flash
                        .lock()
                        .await
                        .erase_sector_4kib((start_sector_i_unoffseted * SECTOR_SIZE) as u32)
                        .await
                        .map_err(VLFSError::from_flash)?;
                    sectors_mng.sector_map.set(start_sector_i as usize, true);
                    sectors_mng.free_sectors_count -= 1;
                    return Ok(start_sector_i_unoffseted as u16);
                }
            }
        }

        defmt::panic!("wtf");
    }

    pub(super) async fn claim_sector(&self, sector_index: u16) {
        let mut sectors_mng = self.sectors_mng.write().await;
        let offsetted_sector_index = sector_index - ALLOC_TABLES_SECTORS_USED as u16;
        if !sectors_mng.sector_map[offsetted_sector_index as usize] {
            sectors_mng.free_sectors_count -= 1;
            sectors_mng
                .sector_map
                .set(offsetted_sector_index as usize, true);
        }
    }

    pub(super) async fn return_sector(&self, sector_index: u16) {
        let mut sectors_mng = self.sectors_mng.write().await;
        let offsetted_sector_index = sector_index - ALLOC_TABLES_SECTORS_USED as u16;
        if sectors_mng.sector_map[offsetted_sector_index as usize] {
            sectors_mng.free_sectors_count += 1;
            sectors_mng
                .sector_map
                .set(offsetted_sector_index as usize, false);
        }
    }
}
