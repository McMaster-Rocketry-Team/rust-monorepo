use crate::{
    io_traits::{AsyncReader, AsyncWriter},
    utils::flash_io::{FlashReader, FlashWriter},
    DummyCrc,
};

use super::*;

// only repersent the state of the file when the struct is created
// does not update after that
#[derive(Debug, Clone, defmt::Format)]
pub struct FileEntry {
    pub opened: bool,
    pub id: FileID,
    pub typ: FileType,
    pub(super) first_sector_index: Option<u16>, // None means the file is empty
}

pub(crate) struct CorruptedFileEntry;

impl FileEntry {
    pub(crate) fn new(file_id: FileID, file_type: FileType) -> Self {
        Self {
            opened: false,
            id: file_id,
            typ: file_type,
            first_sector_index: None,
        }
    }

    pub(super) fn hamming_encode(buffer: [u8; 13]) -> [u8; 13] {
        let mut buffer: BitArray<_, Lsb0> = BitArray::new(buffer);
        buffer.copy_within(57..96, 65);
        buffer.copy_within(26..57, 33);
        buffer.copy_within(11..26, 17);
        buffer.copy_within(4..11, 9);
        buffer.copy_within(1..4, 5);
        buffer.copy_within(0..1, 3);

        buffer.set(0, false);
        for parity_bit_i in 1..8 {
            buffer.set(1 << (parity_bit_i - 1), false);
        }

        let mut parity_bits: BitArray<_, Lsb0> = BitArray::new([0b11111111u8; 1]);
        for bit_i in 1..104 {
            for parity_bit_i in 1..8 {
                if bit_i & (1 << (parity_bit_i - 1)) != 0 {
                    let new_parity_bit = parity_bits[parity_bit_i] ^ buffer[bit_i];
                    parity_bits.set(parity_bit_i, new_parity_bit);
                }
            }
        }
        for parity_bit_i in 1..8 {
            buffer.set(1 << (parity_bit_i - 1), parity_bits[parity_bit_i]);
        }

        let mut parity_bit_whole = true;
        for bit_i in 1..104 {
            parity_bit_whole ^= buffer[bit_i];
        }
        buffer.set(0, parity_bit_whole);

        buffer.into_inner()
    }

    pub(super) fn hamming_decode(buffer: [u8; 13]) -> Result<[u8; 12], ()> {
        let mut buffer: BitArray<_, Lsb0> = BitArray::new(buffer);

        let mut parity_bits: BitArray<_, Lsb0> = BitArray::new([0b11111111u8; 1]);
        for bit_i in 1..104 {
            for parity_bit_i in 1..8 {
                if bit_i & (1 << (parity_bit_i - 1)) != 0 {
                    let new_parity_bit = parity_bits[parity_bit_i] ^ buffer[bit_i];
                    parity_bits.set(parity_bit_i, new_parity_bit);
                }
            }
        }

        let error_i = (parity_bits.into_inner()[0] >> 1) as usize;

        let mut parity_whole = true;
        for bit_i in 0..104 {
            parity_whole ^= buffer[bit_i];
        }

        match (error_i, parity_whole) {
            (0, true) => {
                // whole block parity bit error, do nothing
            }
            (0, false) => {
                // no error, do nothing
            }
            (104..128, true) => {
                // three or more bits error
                return Err(());
            }
            (error_i, true) => {
                // one bit error
                let corrected_bit = !buffer[error_i];
                buffer.set(error_i, corrected_bit);
            }
            (_, false) => {
                // two bit error
                return Err(());
            }
        }

        buffer.copy_within(3..4, 0);
        buffer.copy_within(5..8, 1);
        buffer.copy_within(9..16, 4);
        buffer.copy_within(17..32, 11);
        buffer.copy_within(33..64, 26);
        buffer.copy_within(65..104, 57);

        let mut result = [0u8; 12];
        result.copy_from_slice(&buffer.into_inner()[0..12]);

        Ok(result)
    }

    pub(crate) fn serialize(&self)->[u8;13] {
        let mut buffer = [0u8; 13];
        (&mut buffer[0..2]).copy_from_slice(&self.typ.0.to_be_bytes());
        if let Some(first_sector_index) = self.first_sector_index {
            (&mut buffer[2..4]).copy_from_slice(&first_sector_index.to_be_bytes());
        } else {
            (&mut buffer[2..4]).copy_from_slice(&0xFFFFu16.to_be_bytes());
        }
        (&mut buffer[4..12]).copy_from_slice(&self.id.0.to_be_bytes());

        Self::hamming_encode(buffer)
    }

    // expect a 13 byte buffer
    pub(crate) fn deserialize(buffer: &[u8]) -> Result<Self, CorruptedFileEntry> {
        let buffer = Self::hamming_decode(buffer.try_into().unwrap()).map_err(|_| CorruptedFileEntry)?;

        let file_type = FileType(u16::from_be_bytes((&buffer[0..2]).try_into().unwrap()));
        let first_sector_index = u16::from_be_bytes((&buffer[2..4]).try_into().unwrap());
        let file_id = FileID(u64::from_be_bytes((&buffer[4..12]).try_into().unwrap()) & !1);
        Ok(Self {
            opened: false,
            id: file_id,
            typ: file_type,
            first_sector_index: if first_sector_index == 0xFFFF {
                None
            } else {
                Some(first_sector_index)
            },
        })
    }
}

#[cfg(test)]
mod test {
    use super::FileEntry;

    #[test]
    fn hamming_encode() {
        let data = [0x69u8; 12];
        let mut buffer = [0u8; 13];
        (&mut buffer[0..12]).copy_from_slice(&data);

        let encoded = FileEntry::hamming_encode(buffer);
        // {
        //     use bitvec::prelude::*;
        //     let buffer: BitArray<_, Lsb0> = BitArray::new(encoded.clone());

        //     for bit in buffer.iter() {
        //         print!("{}", if *bit { "1" } else { "0" });
        //     }
        //     println!("");
        // }

        for byte in 0..12 {
            for bit in 0..8 {
                let mut encoded = encoded.clone();
                encoded[byte] ^= 1 << bit;

                let decoded = FileEntry::hamming_decode(encoded).unwrap();

                assert_eq!(data, decoded, "byte {} bit {}", byte, bit);
            }
        }
    }
}

// serialized size must fit in half a block (32kib)
pub(super) struct AllocationTable {
    pub(super) sequence_number: u64,
    pub(super) allocation_table_position: usize, // which half block is the allocation table in
    pub(super) file_count: u16,
    pub(super) max_file_id: FileID,
    pub(super) opened_files: Vec<FileID, 10>,
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self {
            sequence_number: 0,
            allocation_table_position: 0,
            file_count: 0,
            max_file_id: FileID(0),
            opened_files: Vec::new(),
        }
    }
}

impl AllocationTable {
    pub(super) fn address(&self) -> u32 {
        (self.allocation_table_position * 32 * 1024) as u32
    }

    // does not garuntee that the file entry is valid
    pub(super) fn address_of_file_entry(&self, i: u16) -> u32 {
        self.address() + 22 + (i as u32) * 13
    }

    pub(super) fn increment_position(&mut self) {
        self.allocation_table_position = (self.allocation_table_position + 1) % TABLE_COUNT;
        self.sequence_number += 1;
    }

    pub(super) async fn write_header<W: AsyncWriter>(
        &self,
        writer: &mut W,
    ) -> Result<(), W::Error> {
        writer.extend_from_u32(VLFS_VERSION).await?;
        writer.extend_from_u64(self.sequence_number).await?;
        writer.extend_from_u16(self.file_count).await?;
        writer.extend_from_u64(self.max_file_id.0).await?;
        Ok(())
    }
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    // WARNING: this function does not check if the file is already opened
    pub(super) async fn mark_file_opened(
        &self,
        file_id: FileID,
    ) -> Result<(), VLFSError<F::Error>> {
        let mut at = self.allocation_table.write().await;
        at.opened_files
            .push(file_id)
            .map_err(|_| VLFSError::TooManyFilesOpen)?;
        Ok(())
    }

    pub(super) async fn is_file_opened(&self, file_id: FileID) -> bool {
        let at = self.allocation_table.read().await;
        at.opened_files.iter().any(|&id| id == file_id)
    }

    pub(super) async fn mark_file_closed(&self, file_id: FileID) {
        let mut at = self.allocation_table.write().await;
        at.opened_files
            .iter()
            .position(|&id| id == file_id)
            .map(|index| {
                at.opened_files.swap_remove(index);
            });
    }

    pub(super) async fn find_file_entry(
        &self,
        file_id: FileID,
    ) -> Result<Option<(FileEntry, u16)>, VLFSError<F::Error>> {
        let mut flash = self.flash.lock().await;
        let at = self.allocation_table.read().await;
        let file_count = at.file_count;

        let mut buffer = [0u8; 5 + 13];
        let mut dummy_crc = DummyCrc {};
        let mut reader = FlashReader::new(0, &mut flash, &mut dummy_crc);
        let mut left = 0;
        let mut right = file_count - 1;

        while left < right {
            let mid = (left + right) / 2;
            reader.set_address(at.address_of_file_entry(mid));
            let (read_result, _) = reader
                .read_slice(&mut buffer, 13)
                .await
                .map_err(VLFSError::FlashError)?;
            let file_entry = FileEntry::deserialize(read_result)?;
            if file_entry.id < file_id {
                left = mid + 1;
            } else {
                right = mid; // FIXME
            }
        }

        reader.set_address(at.address_of_file_entry(left));
        let (read_result, _) = reader
            .read_slice(&mut buffer, 13)
            .await
            .map_err(VLFSError::FlashError)?;
        let mut file_entry = FileEntry::deserialize(read_result)?;
        if file_entry.id == file_id {
            file_entry.opened = self.is_file_opened(file_id).await;
            return Ok(Some((file_entry, left)));
        } else {
            return Ok(None);
        }
    }

    pub(super) async fn delete_file_entry(
        &self,
        file_id: FileID,
    ) -> Result<(), VLFSError<F::Error>> {
        if let Some((_, file_entry_i)) = self.find_file_entry(file_id).await? {
            let mut at = self.allocation_table.write().await;
            let old_at_address = at.address();
            at.increment_position();
            let at_address = at.address();

            let mut flash = self.flash.lock().await;
            flash
                .erase_block_32kib(at_address)
                .await
                .map_err(VLFSError::FlashError)?;
            drop(flash);

            let mut crc = self.crc.lock().await;
            let mut dummy_crc = DummyCrc {};
            let mut reader_flash = &self.flash;
            let mut writer_flash = &self.flash;
            let mut reader = FlashReader::new(
                old_at_address.try_into().unwrap(),
                &mut reader_flash,
                &mut dummy_crc,
            );
            let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

            // write header to the new allocation table
            at.write_header(&mut writer)
                .await
                .map_err(VLFSError::FlashError)?;

            // copy entries before the deleted entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            let mut buffer = [0u8; 5 + 13];
            for _ in 0..file_entry_i {
                reader
                    .read_slice(&mut buffer, 13)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(&buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            reader.set_address(reader.get_address() + 13);

            // copy entries after the deleted entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            for _ in file_entry_i + 1..at.file_count {
                reader
                    .read_slice(&mut buffer, 13)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(&buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write crc
            writer
                .extend_from_u32(writer.get_crc())
                .await
                .map_err(VLFSError::FlashError)?;

            writer.flush().await.map_err(VLFSError::FlashError)?;

            at.file_count -= 1;
        } else {
            return Err(VLFSError::FileDoesNotExist);
        }
        defmt::todo!()
    }

    pub(super) async fn set_file_first_sector_index(
        &self,
        file_id: FileID,
        first_sector_index: Option<u16>,
    ) -> Result<(), VLFSError<F::Error>> {
        if let Some((mut file_entry, file_entry_i)) = self.find_file_entry(file_id).await? {
            let mut at = self.allocation_table.write().await;
            let old_at_address = at.address();
            at.increment_position();
            let at_address = at.address();

            let mut flash = self.flash.lock().await;
            flash
                .erase_block_32kib(at_address)
                .await
                .map_err(VLFSError::FlashError)?;
            drop(flash);

            let mut crc = self.crc.lock().await;
            let mut dummy_crc = DummyCrc {};
            let mut reader_flash = &self.flash;
            let mut writer_flash = &self.flash;
            let mut reader = FlashReader::new(
                old_at_address.try_into().unwrap(),
                &mut reader_flash,
                &mut dummy_crc,
            );
            let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

            // write header to the new allocation table
            at.write_header(&mut writer)
                .await
                .map_err(VLFSError::FlashError)?;

            // copy entries before the updated entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            let mut buffer = [0u8; 5 + 13];
            for _ in 0..file_entry_i {
                reader
                    .read_slice(&mut buffer, 13)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(&buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write updated file entry
            file_entry.first_sector_index = first_sector_index;
            writer
                .extend_from_slice(&file_entry.serialize())
                .await
                .map_err(VLFSError::FlashError)?;
            reader.set_address(reader.get_address() + 13);

            // copy entries after the updated entry
            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            for _ in file_entry_i + 1..at.file_count {
                reader
                    .read_slice(&mut buffer, 13)
                    .await
                    .map_err(VLFSError::FlashError)?;
                writer
                    .extend_from_slice(&buffer)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            // write crc
            writer
                .extend_from_u32(writer.get_crc())
                .await
                .map_err(VLFSError::FlashError)?;

            writer.flush().await.map_err(VLFSError::FlashError)?;

            return Ok(());
        } else {
            return Err(VLFSError::FileDoesNotExist);
        }
    }

    pub async fn create_file(&self, file_type: FileType) -> Result<FileEntry, VLFSError<F::Error>> {
        let mut at = self.allocation_table.write().await;
        at.max_file_id.increment();
        let old_at_address = at.address();
        at.increment_position();
        at.file_count += 1;
        let at_address = at.address();
        let file_id = at.max_file_id;

        let mut flash = self.flash.lock().await;
        flash
            .erase_block_32kib(at_address)
            .await
            .map_err(VLFSError::FlashError)?;
        drop(flash);

        let mut crc = self.crc.lock().await;
        let mut dummy_crc = DummyCrc {};
        let mut reader_flash = &self.flash;
        let mut writer_flash = &self.flash;
        let mut reader = FlashReader::new(
            old_at_address.try_into().unwrap(),
            &mut reader_flash,
            &mut dummy_crc,
        );
        let mut writer = FlashWriter::new(at_address, &mut writer_flash, &mut crc);

        // write header to the new allocation table
        at.write_header(&mut writer)
            .await
            .map_err(VLFSError::FlashError)?;

        // copy existing file entries
        // TODO optimize this, we can read multiple file entries at once at the expense of more memory
        let mut buffer = [0u8; 5 + 13];
        for _ in 0..at.file_count - 1 {
            reader
                .read_slice(&mut buffer, 13)
                .await
                .map_err(VLFSError::FlashError)?;
            writer
                .extend_from_slice(&buffer)
                .await
                .map_err(VLFSError::FlashError)?;
        }

        // write new file entry
        // there are always even number of 1s in the file entry
        let file_entry = FileEntry::new(file_id, file_type);
        writer
            .extend_from_slice(&file_entry.serialize())
            .await
            .map_err(VLFSError::FlashError)?;

        // write crc
        writer
            .extend_from_u32(writer.get_crc())
            .await
            .map_err(VLFSError::FlashError)?;

        writer.flush().await.map_err(VLFSError::FlashError)?;

        Ok(file_entry)
    }

    // return true: found a valid allocation table
    pub(super) async fn read_latest_allocation_table(&self) -> Result<bool, VLFSError<F::Error>> {
        let mut found_valid_table = false;
        let mut flash = self.flash.lock().await;
        let mut crc = self.crc.lock().await;

        for i in 0..TABLE_COUNT {
            log_info!("Reading allocation table #{}", i + 1);

            let mut read_buffer = [0u8; 5 + 22];
            let mut reader =
                FlashReader::new((i * 32 * 1024).try_into().unwrap(), &mut flash, &mut crc);

            let read_result = reader
                .read_slice(&mut read_buffer, 22)
                .await
                .map_err(VLFSError::FlashError)?
                .0;

            let version = u32::from_be_bytes((&read_result[0..4]).try_into().unwrap());
            let sequence_number = u64::from_be_bytes((&read_result[4..12]).try_into().unwrap());
            let file_count = u16::from_be_bytes((&read_result[12..14]).try_into().unwrap());
            let max_file_id = FileID(u64::from_be_bytes(
                (&read_result[14..22]).try_into().unwrap(),
            ));

            if version != VLFS_VERSION {
                log_warn!(
                    "Version mismatch, expected: {}, actual: {}",
                    VLFS_VERSION,
                    version
                );
                continue;
            }
            if file_count > MAX_FILES as u16 {
                log_warn!("file_count > MAX_FILES");
                continue;
            }

            // TODO optimize this, we can read multiple file entries at once at the expense of more memory
            for _ in 0..file_count {
                reader
                    .read_slice(&mut read_buffer, 13)
                    .await
                    .map_err(VLFSError::FlashError)?;
            }

            let calculated_crc = reader.get_crc();
            let expected_crc = reader
                .read_u32(&mut read_buffer)
                .await
                .map_err(VLFSError::FlashError)?
                .0
                .expect("Reading from flash should always return the desired length");
            if calculated_crc == expected_crc {
                log_info!("CRC match!");
            } else {
                log_warn!(
                    "CRC mismatch! expected: {}, calculated: {}",
                    expected_crc,
                    calculated_crc
                );
                continue;
            }

            let mut at = self.allocation_table.write().await;
            if sequence_number > at.sequence_number {
                at.sequence_number = sequence_number;
                at.allocation_table_position = i;
                at.file_count = file_count;
                at.max_file_id = max_file_id;
                found_valid_table = true;
            }
        }

        return Ok(found_valid_table);
    }

    pub(super) async fn write_empty_allocation_table(&self) -> Result<(), VLFSError<F::Error>> {
        let at = self.allocation_table.read().await;
        let at_address = at.address();

        let mut flash = self.flash.lock().await;
        flash
            .erase_block_32kib(at_address)
            .await
            .map_err(VLFSError::FlashError)?;

        let mut crc = self.crc.lock().await;
        let mut writer = FlashWriter::new(at_address, &mut flash, &mut crc);

        writer
            .extend_from_u32(VLFS_VERSION)
            .await
            .map_err(VLFSError::FlashError)?;
        writer
            .extend_from_u64(at.sequence_number)
            .await
            .map_err(VLFSError::FlashError)?;
        writer
            .extend_from_u16(0)
            .await
            .map_err(VLFSError::FlashError)?;
        writer
            .extend_from_u64(at.max_file_id.0)
            .await
            .map_err(VLFSError::FlashError)?;

        writer
            .extend_from_u32(writer.get_crc())
            .await
            .map_err(VLFSError::FlashError)?;

        writer.flush().await.map_err(VLFSError::FlashError)?;

        Ok(())
    }
}
