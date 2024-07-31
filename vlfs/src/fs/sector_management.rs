use core::mem;

use super::*;

const SECTOR_MAP_ARRAY_SIZE: usize = DATA_REGION_SECTORS / 32;

// false: unused; true: used
pub(crate) struct SectorMap {
    pub(super) map_4k: BitArray<[u32; SECTOR_MAP_ARRAY_SIZE], Lsb0>,
    pub(super) map_32k: BitArray<[u32; (SECTOR_MAP_ARRAY_SIZE / 8) + 1], Lsb0>,
    pub(super) map_64k: BitArray<[u32; (SECTOR_MAP_ARRAY_SIZE / 16) + 1], Lsb0>,
    pub(super) free_sectors_count: u32,
}

impl SectorMap {
    pub(crate) fn new() -> Self {
        Self {
            map_4k: BitArray::default(),
            map_32k: BitArray::default(),
            map_64k: BitArray::default(),
            free_sectors_count: DATA_REGION_SECTORS as u32,
        }
    }

    pub(crate) fn set_sector_used(&mut self, sector_index_unoffsetted: u16) {
        let sector_index = sector_index_unoffsetted as usize - ALLOC_TABLES_SECTORS_USED;
        if self.map_4k[sector_index] {
            return;
        }
        self.map_4k.set(sector_index, true);
        self.map_32k.set(sector_index / 8, true);
        self.map_64k.set(sector_index / 16, true);
        self.free_sectors_count -= 1;
    }

    pub(crate) fn set_sector_unused(&mut self, sector_index_unoffsetted: u16) {
        let sector_index = sector_index_unoffsetted as usize - ALLOC_TABLES_SECTORS_USED;

        if !self.map_4k[sector_index] {
            return;
        }

        self.map_4k.set(sector_index, false);

        let sector_index = sector_index & !0b111;
        let is_32k_used = self.map_4k[sector_index..(sector_index + 8)].any();
        self.map_32k.set(sector_index / 8, is_32k_used);

        let sector_index = sector_index & !0b1111;
        let is_64k_used = self.map_32k[sector_index / 8..(sector_index / 8 + 2)].any();
        self.map_64k.set(sector_index / 16, is_64k_used);

        self.free_sectors_count += 1;
    }
}

pub(crate) struct SectorsMng {
    pub(crate) sector_map: SectorMap,
    pub(crate) erase_ahead_sectors: Vec<u16, 16>,
    pub(crate) async_erase_ahead_sectors: Vec<u16, 16>,
    pub(crate) rng: SmallRng,
}

impl SectorsMng {
    pub(crate) fn new() -> Self {
        Self {
            sector_map: SectorMap::new(),
            erase_ahead_sectors: Vec::new(),
            async_erase_ahead_sectors: Vec::new(),
            rng: SmallRng::seed_from_u64(0),
        }
    }
}

#[derive(Clone, Copy)]
enum EraseLength {
    E64K,
    E32K,
    E4K,
}

impl EraseLength {
    fn get_length_in_sectors(&self) -> u16 {
        match self {
            EraseLength::E64K => 16,
            EraseLength::E32K => 8,
            EraseLength::E4K => 1,
        }
    }
}

// sector index are offseted (does not include allocation table address space)
#[derive(Clone, Copy)]
struct EraseRegion {
    sector_index_offseted: u16,
    length: EraseLength,
}

impl EraseRegion {
    fn get_unoffseted_address(&self) -> u32 {
        (self.sector_index_offseted as u32 + ALLOC_TABLES_SECTORS_USED as u32) * SECTOR_SIZE as u32
    }
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    async fn claim_erase_region(&self, sectors_mng: &mut SectorsMng) -> Result<EraseRegion, ()> {
        if sectors_mng.sector_map.free_sectors_count == 0 {
            return Err(());
        }

        let rng = (sectors_mng.rng.next_u32() / 2) as usize; // divide by 2 to avoid overflow

        {
            // see if it can do 64KiB erase
            for index_64k in 0..(DATA_REGION_SECTORS / 16) {
                let index_64k = (index_64k + rng) % (DATA_REGION_SECTORS / 16);
                if sectors_mng.sector_map.map_64k[index_64k] {
                    continue;
                }
                return Ok(EraseRegion {
                    sector_index_offseted: index_64k as u16 * 16,
                    length: EraseLength::E64K,
                });
            }
        }

        {
            // see if it can do 32KiB erase
            for index_32k in 0..(DATA_REGION_SECTORS / 8) {
                let index_32k = (index_32k + rng) % (DATA_REGION_SECTORS / 8);
                if sectors_mng.sector_map.map_32k[index_32k] {
                    continue;
                }
                return Ok(EraseRegion {
                    sector_index_offseted: index_32k as u16 * 8,
                    length: EraseLength::E32K,
                });
            }
        }

        {
            // fallback to 4KiB erase
            for index_4k in 0..DATA_REGION_SECTORS {
                let index_4k = (index_4k + rng) % DATA_REGION_SECTORS;
                if sectors_mng.sector_map.map_4k[index_4k] {
                    continue;
                }
                return Ok(EraseRegion {
                    sector_index_offseted: index_4k as u16,
                    length: EraseLength::E4K,
                });
            }
        }

        log_unreachable!()
    }

    async fn erase(
        &self,
        region: EraseRegion,
        use_async: bool,
        sectors_mng: &mut SectorsMng,
    ) -> Result<(), VLFSError<F::Error>> {
        let mut flash = self.flash.write().await;

        match region.length {
            EraseLength::E64K => {
                flash
                    .erase_block_64kib(region.get_unoffseted_address())
                    .await
                    .map_err(VLFSError::FlashError)?;
            }
            EraseLength::E32K => {
                flash
                    .erase_block_32kib(region.get_unoffseted_address())
                    .await
                    .map_err(VLFSError::FlashError)?;
            }
            EraseLength::E4K => {
                flash
                    .erase_sector_4kib(region.get_unoffseted_address())
                    .await
                    .map_err(VLFSError::FlashError)?;
            }
        };

        let erase_ahead_sectors: &mut Vec<u16, 16> = if use_async {
            &mut sectors_mng.async_erase_ahead_sectors
        } else {
            &mut sectors_mng.erase_ahead_sectors
        };

        for i in region.sector_index_offseted
            ..(region.sector_index_offseted + region.length.get_length_in_sectors())
        {
            let unoffseted_sector_index = i + ALLOC_TABLES_SECTORS_USED as u16;
            sectors_mng
                .sector_map
                .set_sector_used(unoffseted_sector_index);
            erase_ahead_sectors.push(unoffseted_sector_index).unwrap();
        }
        Ok(())
    }

    pub(super) async fn claim_avaliable_sector_and_erase(
        &self,
    ) -> Result<u16, VLFSError<F::Error>> {
        let mut sectors_mng = self.sectors_mng.write().await;

        if let Some(sector_index) = sectors_mng.erase_ahead_sectors.pop() {
            return Ok(sector_index);
        }

        if !sectors_mng.async_erase_ahead_sectors.is_empty() {
            // swap erase_ahead_sectors and async_erase_ahead_sectors
            unsafe {
                let erase_ahead_sectors = &mut sectors_mng.erase_ahead_sectors as *mut Vec<u16, 16>;
                let async_erase_ahead_sectors =
                    &mut sectors_mng.async_erase_ahead_sectors as *mut Vec<u16, 16>;
                mem::swap(&mut *erase_ahead_sectors, &mut *async_erase_ahead_sectors);
            };

            if let Ok(async_erase_region) = self.claim_erase_region(&mut sectors_mng).await {
                // If using ManagedEraseFlash,
                // this call will start an erase in the background and return immediately.
                self.erase(async_erase_region, true, &mut sectors_mng)
                    .await?;
            }
        } else {
            // both erase_ahead_sectors and async_erase_ahead_sectors are empty
            if let Ok(current_erase_region) = self.claim_erase_region(&mut sectors_mng).await {
                // If using ManagedEraseFlash,
                // this call will start an erase in the background and return immediately.
                self.erase(current_erase_region, false, &mut sectors_mng)
                    .await?;

                if let Ok(async_erase_region) = self.claim_erase_region(&mut sectors_mng).await {
                    // If using ManagedEraseFlash,
                    // this call will wait for the `current_erase_region` erase to finish,
                    // then start an erase for `async_erase_region` in the background and return immediately.
                    self.erase(async_erase_region, true, &mut sectors_mng)
                        .await?;
                }
            }
        }

        sectors_mng
            .erase_ahead_sectors
            .pop()
            .ok_or_else(|| VLFSError::DeviceFull)
    }

    pub(super) async fn claim_sector(&self, sector_index_unoffsetted: u16) {
        let mut sectors_mng = self.sectors_mng.write().await;
        sectors_mng
            .sector_map
            .set_sector_used(sector_index_unoffsetted);
    }

    pub(super) async fn return_sector(&self, sector_index_unoffsetted: u16) {
        let mut sectors_mng = self.sectors_mng.write().await;
        sectors_mng
            .sector_map
            .set_sector_unused(sector_index_unoffsetted);
    }
}
