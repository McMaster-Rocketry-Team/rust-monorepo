pub async fn read_file<'a>(&self, fd: FileDescriptor, buffer: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut opened_files = self.opened_files.write().await;
    if let Some(opened_file) = &mut opened_files[fd] {
        let mut bytes_read: usize = 0;
        let mut current_sector_index = opened_file.file_entry.first_sector_index;
        drop(opened_files);
        let mut flash = self.flash.lock().await;
        while let Some(sector_index) = current_sector_index {
            let sector_address = sector_index as u32 * SECTOR_SIZE as u32;
            let buffer = &mut buffer[bytes_read..];
            flash.read(sector_address, 8, buffer).await;
            let data_length_in_sector = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
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
            let next_sector_index = find_most_common_u16_out_of_4(&buffer[5..13]).unwrap();
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
