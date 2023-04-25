use crate::common::buffer::WriteBuffer;
use crate::driver::flash::{IOReader, SpiReader};
use crate::driver::{crc::Crc, flash::SpiFlash};
use crate::new_write_buffer;
use bitvec::prelude::*;
use defmt::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use heapless::Vec;

use super::rwlock::{RwLock, RwLockReadGuard};

const VLFS_VERSION: u32 = 5;
const SECTORS_COUNT: usize = 16384; // for 512M-bit flash (W25Q512JV)
const SECTOR_SIZE: usize = 4096;
const MAX_FILES: usize = 256; // can be as large as 2728
const TABLE_COUNT: usize = 4;
const FREE_SECTORS_ARRAY_SIZE: usize = SECTORS_COUNT / 32;
const MAX_ALLOC_TABLE_LENGTH: usize = 16 + MAX_FILES * 12;
const MAX_OPENED_FILES: usize = 10;
const WRITING_QUEUE_SIZE: usize = 4;

#[derive(Debug, Clone)]
struct FileEntry {
    file_id: u64,
    file_type: u16,
    first_sector_index: Option<u16>, // None means the file is empty
}

// serialized size must fit in half a block (32kib)
struct AllocationTable {
    sequence_number: u32,
    file_entries: Vec<FileEntry, MAX_FILES>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            file_entries: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct OpenedFile {
    current_sector_index: Option<u16>, // None means the file is empty
    file_entry: FileEntry,             // FIXME use reference
}

type FileDescriptor = usize;

pub struct WritingQueueEntry {
    pub fd: FileDescriptor,
    pub data: [u8; 5 + SECTOR_SIZE - 256],
    pub data_length: u16,
    pub overwrite_sector: bool,
}

impl WritingQueueEntry {
    pub fn new(fd: FileDescriptor) -> Self {
        Self {
            fd,
            data: [0xFFu8; 5 + SECTOR_SIZE - 256],
            data_length: 0,
            overwrite_sector: false,
        }
    }
}

struct AllocationTableWrapper {
    allocation_table: AllocationTable,
    allocation_table_index: usize, // which half block is the allocation table in
}

impl Default for AllocationTableWrapper {
    fn default() -> Self {
        Self {
            allocation_table: AllocationTable::default(),
            allocation_table_index: TABLE_COUNT - 1,
        }
    }
}

pub struct VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    allocation_table: RwLock<CriticalSectionRawMutex, AllocationTableWrapper, 10>,
    free_sectors: BitArray<[u32; FREE_SECTORS_ARRAY_SIZE], Lsb0>,
    flash: Mutex<CriticalSectionRawMutex, F>,
    crc: Mutex<CriticalSectionRawMutex, C>,
    opened_files: Vec<Option<OpenedFile>, MAX_OPENED_FILES>,
    writing_queue: Channel<CriticalSectionRawMutex, WritingQueueEntry, WRITING_QUEUE_SIZE>,
}

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub fn new(flash: F, crc: C) -> Self {
        let mut opened_files = Vec::<Option<OpenedFile>, MAX_OPENED_FILES>::new();
        for _ in 0..MAX_OPENED_FILES {
            opened_files.push(None).unwrap();
        }
        let mut free_sectors = BitArray::<_, Lsb0>::new([0u32; FREE_SECTORS_ARRAY_SIZE]);
        for i in 0..(TABLE_COUNT * 32 * 1024 / SECTOR_SIZE) {
            free_sectors.set(i, true);
        }
        Self {
            allocation_table: RwLock::new(AllocationTableWrapper::default()),
            free_sectors,
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
            opened_files,
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
        let free_sectors = self.free_sectors.as_mut_bitslice();
        let at = self.allocation_table.read().await;
        for file_entry in &at.allocation_table.file_entries {
            let mut current_sector_index = file_entry.first_sector_index;
            while let Some(sector_index) = current_sector_index {
                free_sectors.set(sector_index as usize, true);
                used_sectors += 1;

                let mut buffer = [0u8; 5 + 8];
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
                self.flash
                    .get_mut()
                    .read(sector_address + 4096 - 256, 8, &mut buffer)
                    .await;
                let a = u16::from_be_bytes((&buffer[5..7]).try_into().unwrap());
                let b = u16::from_be_bytes((&buffer[7..9]).try_into().unwrap());
                let c = u16::from_be_bytes((&buffer[9..11]).try_into().unwrap());
                let d = u16::from_be_bytes((&buffer[11..13]).try_into().unwrap());
                let next_sector_index = Self::find_most_common(a, b, c, d).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
            }
        }

        used_sectors
    }

    pub async fn files_iter(&self) -> FilesIterator {
        FilesIterator {
            i: 0,
            at: self.allocation_table.read().await,
        }
    }

    pub async fn create_file(&mut self, file_id: u64, file_type: u16) -> Result<(), ()> {
        let mut at = self.allocation_table.write().await;
        for file_entry in &at.allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Err(());
            }
        }

        let file_entry = FileEntry {
            file_id,
            file_type,
            first_sector_index: None,
        };
        at.allocation_table.file_entries.push(file_entry).unwrap(); // TODO error handling
        drop(at);
        self.write_allocation_table().await;
        Ok(())
    }

    pub async fn remove_file(&mut self, file_id: u64) -> Result<(), ()> {
        for opened_file in &self.opened_files {
            if let Some(opened_file) = opened_file {
                if opened_file.file_entry.file_id == file_id {
                    return Err(());
                }
            }
        }

        let mut at = self.allocation_table.write().await;
        for i in 0..at.allocation_table.file_entries.len() {
            if at.allocation_table.file_entries[i].file_id == file_id {
                at.allocation_table.file_entries.remove(i);
                break;
            }
        }
        drop(at);
        self.write_allocation_table().await;
        // TODO update sectors list
        Ok(())
    }

    fn find_most_common(a: u16, b: u16, c: u16, d: u16) -> Option<u16> {
        if a == b {
            return Some(a);
        }
        if a == c {
            return Some(a);
        }
        if a == d {
            return Some(a);
        }
        if b == c {
            return Some(b);
        }
        if b == d {
            return Some(b);
        }
        if c == d {
            return Some(c);
        }

        None
    }

    pub async fn get_file_size(&self, file_id: u64) -> Option<(usize, usize)> {
        let at = self.allocation_table.read().await;
        if let Some(file_entry) = self.find_file_entry(&at.allocation_table, file_id) {
            let mut size: usize = 0;
            let mut sectors: usize = 0;
            let mut current_sector_index = file_entry.first_sector_index;
            let mut buffer = [0u8; 5 + 8];
            while let Some(sector_index) = current_sector_index {
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;

                // read data length
                let mut flash = self.flash.lock().await;
                flash.read(sector_address, 8, &mut buffer).await;
                let a = u16::from_be_bytes((&buffer[5..7]).try_into().unwrap());
                let b = u16::from_be_bytes((&buffer[7..9]).try_into().unwrap());
                let c = u16::from_be_bytes((&buffer[9..11]).try_into().unwrap());
                let d = u16::from_be_bytes((&buffer[11..13]).try_into().unwrap());
                size += Self::find_most_common(a, b, c, d).unwrap() as usize;

                // read next sector index
                flash
                    .read(sector_address + 4096 - 256, 8, &mut buffer)
                    .await;
                let a = u16::from_be_bytes((&buffer[5..7]).try_into().unwrap());
                let b = u16::from_be_bytes((&buffer[7..9]).try_into().unwrap());
                let c = u16::from_be_bytes((&buffer[9..11]).try_into().unwrap());
                let d = u16::from_be_bytes((&buffer[11..13]).try_into().unwrap());
                let next_sector_index = Self::find_most_common(a, b, c, d).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };
                sectors += 1;
            }

            return Some((size, sectors));
        }

        None
    }

    fn find_file_entry<'a>(
        &self,
        allocation_table: &'a AllocationTable,
        file_id: u64,
    ) -> Option<&'a FileEntry> {
        for file_entry in &allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Some(file_entry);
            }
        }
        None
    }

    pub async fn open_file(&mut self, file_id: u64) -> Option<FileDescriptor> {
        for opened_file in &self.opened_files {
            if let Some(opened_file) = opened_file {
                if opened_file.file_entry.file_id == file_id {
                    return None;
                }
            }
        }

        let file_descriptor = self.find_avaliable_file_descriptor()?;
        let at = self.allocation_table.read().await;
        if let Some(file_entry) = self.find_file_entry(&at.allocation_table, file_id) {
            let opened_file = OpenedFile {
                current_sector_index: file_entry.first_sector_index,
                file_entry: file_entry.clone(),
            };
            self.opened_files[file_descriptor].replace(opened_file);
            return Some(file_descriptor);
        }

        None
    }

    pub async fn write_file(&mut self, data: WritingQueueEntry) {
        self.writing_queue.send(data).await;
    }

    pub async fn read_file<'a>(
        &mut self,
        fd: FileDescriptor,
        buffer: &'a mut [u8],
    ) -> Option<&'a [u8]> {
        if let Some(opened_file) = &mut self.opened_files[fd] {
            let mut bytes_read: usize = 0;
            let flash = self.flash.get_mut();
            let mut current_sector_index = opened_file.file_entry.first_sector_index;
            while let Some(sector_index) = current_sector_index {
                let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
                let buffer = &mut buffer[bytes_read..];
                flash.read(sector_address, 8, buffer).await;
                let data_length_in_sector = Self::find_most_common(
                    u16::from_be_bytes((&buffer[5..7]).try_into().unwrap()),
                    u16::from_be_bytes((&buffer[7..9]).try_into().unwrap()),
                    u16::from_be_bytes((&buffer[9..11]).try_into().unwrap()),
                    u16::from_be_bytes((&buffer[11..13]).try_into().unwrap()),
                )
                .unwrap();
                let data_length_in_sector_padded = (data_length_in_sector + 3) & !3;

                flash
                    .read(
                        sector_address + 8,
                        data_length_in_sector_padded as usize + 4,
                        buffer,
                    )
                    .await;
                let crc_actual = self
                    .crc
                    .lock()
                    .await
                    .calculate(&buffer[5..(data_length_in_sector_padded as usize + 5)]);
                let crc_expected = u32::from_be_bytes(
                    (&buffer[(data_length_in_sector_padded as usize + 5)
                        ..(data_length_in_sector_padded as usize + 9)])
                        .try_into()
                        .unwrap(),
                );
                if crc_actual != crc_expected {
                    warn!(
                        "CRC mismatch: expected {}, got {}",
                        crc_expected, crc_actual
                    );
                    return None;
                }

                // read next sector index
                let buffer = &mut buffer[data_length_in_sector as usize..];
                flash.read(sector_address + 4096 - 256, 8, buffer).await;
                let a = u16::from_be_bytes((&buffer[5..7]).try_into().unwrap());
                let b = u16::from_be_bytes((&buffer[7..9]).try_into().unwrap());
                let c = u16::from_be_bytes((&buffer[9..11]).try_into().unwrap());
                let d = u16::from_be_bytes((&buffer[11..13]).try_into().unwrap());
                let next_sector_index = Self::find_most_common(a, b, c, d).unwrap();
                current_sector_index = if next_sector_index == 0xFFFF {
                    None
                } else {
                    Some(next_sector_index)
                };

                bytes_read += data_length_in_sector as usize;
            }

            return Some(&buffer[5..(5 + bytes_read)]);
        }

        None
    }

    async fn flush_single(&mut self, mut entry: WritingQueueEntry) {
        let new_sector_index = self.find_avaliable_sector().unwrap(); // FIXME handle error

        if let Some(opened_file) = &mut self.opened_files[entry.fd] {
            // save the new sector index to the end of the last sector
            let was_empty = if let Some(last_sector_index) = opened_file.current_sector_index {
                let last_sector_next_sector_address =
                    (last_sector_index as usize * SECTOR_SIZE + (4096 - 256)) as u32;
                new_write_buffer!(write_buffer, 256);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                write_buffer.extend_from_u16(new_sector_index);
                self.flash
                    .lock()
                    .await
                    .write_256b(last_sector_next_sector_address, write_buffer.as_mut_slice())
                    .await;
                false
            } else {
                let mut at = self.allocation_table.write().await;
                for file_entry in &mut at.allocation_table.file_entries {
                    if file_entry.file_id == opened_file.file_entry.file_id {
                        opened_file.file_entry.first_sector_index = Some(new_sector_index);
                        file_entry.first_sector_index = Some(new_sector_index);
                        break;
                    }
                }
                true
            };

            // write the data to new sector
            let write_sector_address = if entry.overwrite_sector && let Some(current_sector_index) = opened_file.current_sector_index {
                (current_sector_index as usize * SECTOR_SIZE) as u32
            } else {
                self.free_sectors
                    .as_mut_bitslice()
                    .set(new_sector_index as usize, true);
                opened_file.current_sector_index = Some(new_sector_index);
                (new_sector_index as usize * SECTOR_SIZE) as u32
            };

            if was_empty {
                self.write_allocation_table().await;
            }

            // erase old data
            let mut flash = self.flash.lock().await;
            flash.erase_sector_4kib(write_sector_address).await;

            // put crc to the buffer
            {
                let mut write_buffer = WriteBuffer::new(&mut entry.data, 5 + 8);
                write_buffer.set_offset(entry.data_length as usize);
                write_buffer.align_4_bytes();
                let mut crc = self.crc.lock().await;
                let crc = crc.calculate(write_buffer.as_slice_without_start());
                write_buffer.extend_from_u32(crc);
            }

            // put length of the data to the buffer
            {
                let mut write_buffer = WriteBuffer::new(&mut entry.data, 5);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
                write_buffer.extend_from_u16(entry.data_length);
            }

            // write buffer to flash
            flash
                .write(
                    write_sector_address,
                    (8 + entry.data_length + 4) as usize,
                    &mut entry.data,
                )
                .await;
        }
    }

    pub async fn flush(&mut self) {
        loop {
            let data = self.writing_queue.receiver().try_recv();
            let entry = if data.is_err() {
                return;
            } else {
                data.unwrap()
            };

            self.flush_single(entry).await;
        }
    }

    // TODO optimize
    fn find_avaliable_sector(&self) -> Option<u16> {
        for i in 0..SECTORS_COUNT {
            if !self.free_sectors[i] {
                return Some(i.try_into().unwrap());
            }
        }
        None
    }

    fn find_avaliable_file_descriptor(&self) -> Option<FileDescriptor> {
        for fd in 0..MAX_OPENED_FILES {
            if self.opened_files[fd].is_none() {
                return Some(fd);
            }
        }
        None
    }

    async fn read_allocation_table(&mut self) -> bool {
        let mut found_valid_table = false;
        let mut at = self.allocation_table.write().await;

        for i in 0..TABLE_COUNT {
            info!("Reading allocation table #{}", i + 1);

            let flash = self.flash.get_mut();
            let crc = self.crc.get_mut();
            let mut reader = SpiReader::new((i * 32 * 1024).try_into().unwrap(), flash, crc);

            reader.reset_crc();
            let version = reader.read_u32().await;
            if version != VLFS_VERSION {
                warn!(
                    "Version mismatch for allocation table #{}, expected: {}, actual: {}",
                    i, VLFS_VERSION, version
                );
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
                let file_id = reader.read_u64().await;
                let file_type = reader.read_u16().await;
                let first_sector_index = reader.read_u16().await;
                files
                    .push(FileEntry {
                        file_id,
                        file_type,
                        first_sector_index: if first_sector_index == 0xFFFF {
                            None
                        } else {
                            Some(first_sector_index)
                        },
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

    async fn write_allocation_table(&self) {
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

#[derive(Debug, Clone)]
pub struct LsFileEntry {
    pub file_id: u64,
    pub file_type: u16,
}

pub struct FilesIterator<'a> {
    i: usize,
    at: RwLockReadGuard<'a, CriticalSectionRawMutex, AllocationTableWrapper, 10>,
}

impl<'a> FilesIterator<'a> {
    pub fn len(&self) -> usize {
        self.at.allocation_table.file_entries.len()
    }
}

impl<'a> Iterator for FilesIterator<'a> {
    type Item = LsFileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .at
            .allocation_table
            .file_entries
            .get(self.i)
            .map(|file_entry| LsFileEntry {
                file_id: file_entry.file_id,
                file_type: file_entry.file_type,
            });
        self.i += 1;
        result
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
