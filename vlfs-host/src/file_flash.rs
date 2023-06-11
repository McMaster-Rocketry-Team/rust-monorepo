use std::path;

use random_access_disk::RandomAccessDisk;
use random_access_storage::{RandomAccess, RandomAccessError};
use vlfs::Flash;

pub struct FileFlash {
    rad: RandomAccessDisk,
}

const SIZE: u32 = 262144 * 256;

impl FileFlash {
    pub async fn new(file_name: path::PathBuf) -> Result<Self, RandomAccessError> {
        let mut rad = RandomAccessDisk::open(file_name).await?;
        rad.truncate(SIZE as u64).await?;
        Ok(Self { rad })
    }
}

#[derive(defmt::Format, Debug)]
pub struct RandomAccessErrorWrapper(#[defmt(Debug2Format)] RandomAccessError);

impl From<RandomAccessError> for RandomAccessErrorWrapper {
    fn from(e: RandomAccessError) -> Self {
        Self(e)
    }
}

// WARNING: This implementation is not compatible with flash dumps from the real hardware!
impl Flash for FileFlash {
    type Error = RandomAccessErrorWrapper;

    fn size(&self) -> u32 {
        SIZE
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.rad.write(address as u64, &[0u8; 4 * 1024]).await?;
        Ok(())
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.rad.write(address as u64, &[0u8; 32 * 1024]).await?;
        Ok(())
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.rad.write(address as u64, &[0u8; 64 * 1024]).await?;
        Ok(())
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        let read_result = self.rad.read(address as u64, read_length as u64).await?;
        (&mut read_buffer[5..(read_length + 5)]).copy_from_slice(&read_result[..read_length]);
        Ok(&read_buffer[5..(read_length + 5)])
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.rad.write(address as u64, &write_buffer[5..]).await?;
        Ok(())
    }
}