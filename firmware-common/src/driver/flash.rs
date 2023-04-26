use defmt::*;

use crate::common::io_traits::AsyncReader;

use super::crc::Crc;

pub trait SpiFlash {
    // size in bytes
    fn size(&self) -> u32;

    async fn erase_sector_4kib(&mut self, address: u32);
    async fn erase_block_32kib(&mut self, address: u32);
    async fn erase_block_64kib(&mut self, address: u32);

    // maximum read length is 4 kb
    // size of the buffer must be at least 5 bytes larger than read_length
    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> &'b [u8];

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be less than or equals 261 bytes long, where the last 256 bytes are the data to write
    async fn write_256b<'b>(&mut self, address: u32, write_buffer: &'b mut [u8]);

    // read arbitary length, length of read_buffer must be larger or equal to read_length + 5
    async fn read<'b, 'c>(&mut self, address: u32, read_length: usize, read_buffer: &'b mut [u8]) {
        let mut bytes_read = 0;
        while bytes_read < read_length {
            let length = if read_length - bytes_read > 4096 {
                4096
            } else {
                read_length - bytes_read
            };
            info!("reading {}/{} bytes", bytes_read, read_length);
            self.read_4kib(
                address + bytes_read as u32,
                length,
                &mut read_buffer[bytes_read..],
            )
            .await;
            bytes_read += length;
        }
    }

    // write arbitary length (must be a multiple of 256 bytes)
    // address must be 256-byte-aligned
    // length of write_buffer must be larger or equal to read_length + 5
    async fn write<'b, 'c>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) {
        let mut bytes_written = 0;
        while bytes_written < write_length {
            let length = if write_length - bytes_written > 256 {
                256
            } else {
                write_length - bytes_written
            };
            info!("writing {}/{} bytes", bytes_written, write_length);
            self.write_256b(
                address + bytes_written as u32,
                &mut write_buffer[bytes_written..],
            )
            .await;
            bytes_written += length;
        }
    }
}

pub struct SpiReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    address: u32,
    flash: &'a mut F,
    crc: &'a mut C,
    crc_buffer: [u8; 4],
    crc_buffer_index: usize,
}

impl<'a, 'b, F, C> SpiReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub fn new(start_address: u32, flash: &'a mut F, crc: &'a mut C) -> Self {
        crc.reset();
        Self {
            address: start_address,
            flash,
            crc,
            crc_buffer: [0; 4],
            crc_buffer_index: 0,
        }
    }

    pub fn reset_crc(&mut self) {
        self.crc.reset();
    }

    pub fn get_crc(&self) -> u32 {
        self.crc.read()
    }
}

impl<'a, F, C> AsyncReader for SpiReader<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    // maximum read length is the length of buffer - 5 bytes
    async fn read_slice<'b>(&mut self, buffer: &'b mut [u8], length: usize) -> &'b [u8] {
        self.flash
            .read_4kib(self.address, length, buffer)
            .await;

        self.address += length as u32;

        for i in 5..(length + 5) {
            self.crc_buffer[self.crc_buffer_index] = buffer[i];
            self.crc_buffer_index += 1;
            if self.crc_buffer_index == 4 {
                self.crc_buffer_index = 0;
                self.crc.feed(u32::from_be_bytes(self.crc_buffer));
            }
        }

        &buffer[5..(length + 5)]
    }
}
