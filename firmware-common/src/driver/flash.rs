use core::ops::{Deref, DerefMut};

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

use super::{bus_error::BusError, spi::SpiBusError};

pub trait Flash {
    type Error: defmt::Format;

    // size in bytes
    fn size(&self) -> u32;

    async fn reset(&mut self) -> Result<(), Self::Error>;

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error>;

    // maximum read length is 4 kb
    // size of the buffer must be at least 5 bytes larger than read_length
    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error>;

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be less than or equals 261 bytes long, where the last 256 bytes are the data to write
    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error>;

    // read arbitary length, length of read_buffer must be larger or equal to read_length + 5
    async fn read<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        let mut bytes_read = 0;
        while bytes_read < read_length {
            let length = if read_length - bytes_read > 4096 {
                4096
            } else {
                read_length - bytes_read
            };
            // info!("reading {}/{} bytes", bytes_read, read_length);
            self.read_4kib(
                address + bytes_read as u32,
                length,
                &mut read_buffer[bytes_read..],
            )
            .await?;
            bytes_read += length;
        }

        Ok(&read_buffer[5..(5 + read_length)])
    }

    // write arbitary length (must be a multiple of 256 bytes)
    // address must be 256-byte-aligned
    // length of write_buffer must be larger or equal to write_length + 5
    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        let mut bytes_written = 0;
        while bytes_written < write_length {
            let length = if write_length - bytes_written > 256 {
                256
            } else {
                write_length - bytes_written
            };
            // info!("writing {}/{} bytes", bytes_written, write_length);
            self.write_256b(
                address + bytes_written as u32,
                &mut write_buffer[bytes_written..],
            )
            .await?;
            bytes_written += length;
        }

        Ok(())
    }
}

impl<'a, M, T> Flash for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Flash,
{
    type Error = T::Error;

    fn size(&self) -> u32 {
        self.deref().size()
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.deref_mut().reset().await
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_sector_4kib(address).await
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_32kib(address).await
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_64kib(address).await
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        self.deref_mut()
            .read_4kib(address, read_length, read_buffer)
            .await
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.deref_mut().write_256b(address, write_buffer).await
    }
}
