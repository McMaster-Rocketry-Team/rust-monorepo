use std::path;

use vlfs::Flash;

const SIZE: u32 = 262144 * 256;

#[derive(Clone)]
pub struct MemoryFlash {
    file_name: Option<path::PathBuf>,
    buffer: Vec<u8>,
}

impl MemoryFlash {
    pub fn new(file_name: Option<path::PathBuf>) -> Self {
        let buffer = vec![0; SIZE as usize];
        Self { file_name, buffer }
    }
}

impl Flash for MemoryFlash {
    type Error = ();

    async fn size(&self) -> u32 {
        SIZE
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        (&mut self.buffer[(address as usize)..(address as usize + 4 * 1024)])
            .copy_from_slice(&[0xFFu8; 4 * 1024]);
        Ok(())
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        (&mut self.buffer[(address as usize)..(address as usize + 32 * 1024)])
            .copy_from_slice(&[0xFFu8; 32 * 1024]);
        Ok(())
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        (&mut self.buffer[(address as usize)..(address as usize + 64 * 1024)])
            .copy_from_slice(&[0xFFu8; 64 * 1024]);
        Ok(())
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        (&mut read_buffer[5..(read_length + 5)])
            .copy_from_slice(&self.buffer[(address as usize)..(address as usize + read_length)]);
        Ok(&read_buffer[5..(read_length + 5)])
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        println!("write_256b: address={:#X}", address);
        println!("{:02X?}", &write_buffer[5..]);
        (&mut self.buffer[(address as usize)..(address as usize + write_buffer.len() - 5)])
            .copy_from_slice(&write_buffer[5..]);
        Ok(())
    }
}

impl Drop for MemoryFlash {
    fn drop(&mut self) {
        if let Some(file_name) = &self.file_name {
            std::fs::create_dir_all(file_name.parent().unwrap()).unwrap();
            std::fs::write(file_name, &self.buffer).unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn read_write() {
        let mut flash = MemoryFlash::new(None);
        let mut buffer = [0u8; 10 + 5];
        buffer[5..].copy_from_slice(&[0x01; 10]);
        flash.write_256b(256, &mut buffer).await.unwrap();
        let mut read_buffer = [0u8; 10 + 5];
        let read_buffer = flash.read_4kib(256, 10, &mut read_buffer).await.unwrap();
        assert_eq!(&buffer[5..], read_buffer);
    }
}
