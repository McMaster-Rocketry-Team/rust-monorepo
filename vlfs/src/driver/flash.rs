use core::{
    fmt::Debug,
    future::Future,
    ops::{Deref, DerefMut},
};

/// A Flash driver is represented here
pub trait Flash {
    // Error type that will be returned by the flash driver.
    // This type must implement the defmt::Format trait.
    type Error: defmt::Format + Debug + embedded_io_async::Error;

    // This function returns the size of the flash memory in bytes.
    fn size(&self) -> impl Future<Output = u32>;

    // "reboots" the flash device, returning it to a known state
    fn reset(&mut self) -> impl Future<Output = Result<(), Self::Error>>;

    // erase methods must set all the erased bits to 1 - VLFS relies on this assumption
    fn erase_sector_4kib(&mut self, address: u32) -> impl Future<Output = Result<(), Self::Error>>;
    fn erase_block_32kib(&mut self, address: u32) -> impl Future<Output = Result<(), Self::Error>>;
    fn erase_block_64kib(&mut self, address: u32) -> impl Future<Output = Result<(), Self::Error>>;

    /// Reads 4KiB of data from the memory device starting at the specified address.
    /// maximum read length is 4 kb
    /// size of the buffer must be at least 5 bytes larger than read_length
    ///
    /// #Arguments
    ///
    /// * `address` - u32 integer that specifies the starting address of the 4KiB block that will be read
    /// * `read_length` - usize integer that specifies the # of bytes to be read from the memory device.
    /// * `read_buffer` - buffer where read data will be stored
    ///
    /// #Examples
    ///
    /// ```
    /// let mut read_buffer = [0u8; 5 + 4096];
    /// let read_data = flash.read_4kib(0x0000_0000, 4096, &mut read_buffer).await;
    ///
    /// ```
    fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> impl Future<Output = Result<&'b [u8], Self::Error>>; //

    /// Writes 256 bytes of data to the memory device starting at the specified address.
    /// size of the buffer must be at least 261 bytes long
    ///
    /// #Arguments
    ///
    /// * `address` - u32 integer that specifies the starting address of the 256 byte block that will be written
    /// * `write_buffer` - buffer where data to be written is stored
    ///
    /// #Examples
    ///
    /// ```
    /// let mut write_buffer = [0u8; 261];
    /// flash.write_256b(0x0000_0000, &mut write_buffer).await;
    ///
    /// ```
    fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Reads arbitary length of data from the memory device starting at the specified address.
    /// maximum read length is 4 kb
    /// size of the buffer must be at least read_length + 5 bytes long
    /// refer to the read_4kib function for more details on the parameters and outputs as they are the same
    async fn read<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        /* This loop reads the data from the flash by calling read_4kib multiple times, with each call
        reading up to 4096 bytes at a time, until it has read the entire requested length.

        Variables:
           bytes_read: usize integer that keeps track of the number of bytes that have been read so far
           length:     usize integer that specifies the number of bytes to be read in the current iteration */

        let mut bytes_read = 0;
        while bytes_read < read_length {
            // Determine the maximum number of bytes to read in this iteration, which is either the remaining
            // number of bytes to read or the maximum of 4096.
            let length = if read_length - bytes_read > 4096 {
                4096
            } else {
                read_length - bytes_read
            };
            // Call read_4kib with the current address and length to read, and store the result in the read_buffer
            // at the appropriate location.
            self.read_4kib(
                address + bytes_read as u32,
                length,
                &mut read_buffer[bytes_read..],
            )
            .await?;
            // Update the number of bytes read.
            bytes_read += length;
        }

        // Return the slice of the read_buffer that contains the actual data. This is done by taking a slice of the read_buffer
        // from the 5th byte to the 5th byte + the read_length. The first 5 bytes are skipped because they are used for the spi command
        Ok(&read_buffer[5..(5 + read_length)])
    }

    /// write arbitary length (must be a multiple of 256 bytes)
    /// address must be 256-byte-aligned
    /// length of write_buffer must be larger or equal to write_length + 5
    /// refer to the write_256b function for more details on the parameters and outputs as they are the same
    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        /* This loop writes the data to the flash by calling write_256b multiple times, with each call
        writing up to 256 bytes at a time, until it has written the entire requested length.

        Variables:
        bytes_written: usize integer that keeps track of the number of bytes that have been written so far
        length:        usize integer that specifies the number of bytes to be written in the current iteration */

        let mut bytes_written = 0;
        while bytes_written < write_length {
            let length = if write_length - bytes_written > 256 {
                256
            } else {
                write_length - bytes_written
            };

            self.write_256b(
                address + bytes_written as u32,
                &mut write_buffer[bytes_written..],
            )
            .await?;
            // Update the number of bytes written.
            bytes_written += length;
        }
        // Return Ok if no error occured during all the write operations
        Ok(())
    }
}

impl<F: Flash, T: DerefMut<Target = F> + Deref<Target = F>> Flash for T {
    type Error = F::Error;

    async fn size(&self) -> u32 {
        self.deref().size().await
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
