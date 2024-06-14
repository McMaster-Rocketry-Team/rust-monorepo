use crate::Flash;

#[derive(defmt::Format)]
pub struct DummyFlash;

#[derive(defmt::Format, Debug)]
pub struct DummyFlashError;

impl embedded_io_async::Error for DummyFlashError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Other
    }
}

impl Flash for DummyFlash {
    type Error = DummyFlashError;

    async fn size(&self) -> u32 {
        262144 * 256
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_sector_4kib(&mut self, _address: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_block_32kib(&mut self, _address: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_block_64kib(&mut self, _address: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read_4kib<'b>(
        &mut self,
        _address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        Ok(&read_buffer[..read_length])
    }

    async fn write_256b<'b>(
        &mut self,
        _address: u32,
        _write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
