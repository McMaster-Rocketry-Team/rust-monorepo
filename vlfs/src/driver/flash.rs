
use core::ops::{Deref, DerefMut};
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

/// A Flash driver is represented here
pub trait Flash {
    // Error type that will be returned by the flash driver.
    // This type must implement the defmt::Format trait.
    type Error: defmt::Format + Debug;

    // This function returns the size of the flash memory in bytes.
    fn size(&self) -> u32;

    // "reboots" the flash device, returning it to a known state
    async fn reset(&mut self) -> Result<(), Self::Error>;

    // erase methods must set all the erased bits to 1 - VLFS relies on this assumption
    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error>;

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
    /// 
    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error>; //

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
    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error>;


    /// Reads arbitary length of data from the memory device starting at the specified address.
    /// maximum read length is 4 kb
    /// size of the buffer must be at least read_length + 5 bytes long
    /// refer to the read_4kib function for more details on the parameters and outputs as they are the same
    /// 

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

    /// write arbitrary length (must be a multiple of 256 bytes)
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
        // Return Ok if no error occurred during all the write operations
        Ok(())
    }
}

// `MutexGuard` is a type from the `embassy-sync` crate that represents a held lock on a mutex
// This implementation of `Flash` is for a flash memory that can be accessed through a mutex
impl<'a, M, T> Flash for MutexGuard<'a, M, T>
where
    M: RawMutex, // M is a type that implements the `RawMutex` trait
    T: Flash, // T is a type that implements the `Flash` trait. This is the type of the wrapped object
{
    // `Error` is the error type of the wrapped `T` object
    type Error = T::Error;

    // `size` returns the total size of the flash memory, in bytes
    fn size(&self) -> u32 {
        self.deref().size()
    }

    // `reset` erases all contents of the flash memory
    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.deref_mut().reset().await
    }

    // `erase_sector_4kib` erases a 4KB sector of the flash memory starting at the given address
    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_sector_4kib(address).await
    }

    // `erase_block_32kib` erases a 32KB block of the flash memory starting at the given address
    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_32kib(address).await
    }

    // `erase_block_64kib` erases a 64KB block of the flash memory starting at the given address
    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_64kib(address).await
    }

    // `read_4kib` reads a 4KB block of the flash memory starting at the given address into the provided buffer
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
    // `write_256b` writes a 256B block of the flash memory starting at the given address from the provided buffer
    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.deref_mut().write_256b(address, write_buffer).await
    }
}

#[cfg(test)]
mod tests {
    use super::{Flash, MutexGuard, *};
    use embassy_sync::blocking_mutex::Mutex;
    use futures::Future;
    //import VLFS
    use crate::fs::VLFS;
    use crate::driver::

    struct MockFlash {
        data: Vec<u8>,
    }
    struct MockCrc; //ToDo: implement crc trait

    #[derive(Debug)] // make MockFlashError printable
    struct MockFlashError;

    impl defmt::Format for MockFlashError {
        fn format(&self, f: defmt::Formatter) {
            defmt::write!(f, "MockFlashError");
        }
    }

    impl Flash for MockFlash {
        type Error = MockFlashError;

        fn size(&self) -> u32 {
            self.data.len() as u32
        }

        async fn reset(&mut self) -> Result<(), Self::Error> {
            self.data.fill(0xFF);
            Ok(())
        }

        async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
            if address as usize >= self.data.len() {
                return Err(MockFlashError);
            }

            let end = std::cmp::min(self.data.len(), (address + 4096) as usize);
            for byte in self.data[address as usize..end].iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
            if address as usize >= self.data.len() {
                return Err(MockFlashError);
            }

            let end = std::cmp::min(self.data.len(), (address + 32768) as usize);
            for byte in self.data[address as usize..end].iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
            if address as usize >= self.data.len() {
                return Err(MockFlashError);
            }

            let end = std::cmp::min(self.data.len(), (address + 65536) as usize);
            for byte in self.data[address as usize..end].iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        async fn read_4kib<'b>(
            &mut self,
            address: u32,
            read_length: usize,
            read_buffer: &'b mut [u8],
        ) -> Result<&'b [u8], Self::Error> {
            if address as usize + read_length > self.data.len() {
                Err(MockFlashError)
            }
            // Get the requested slice from the data vector.
            let data_slice = &self.data[address as usize..address as usize + read_length];

            // Copy the data slice into the read buffer.
            read_buffer[..data_slice.len()].copy_from_slice(data_slice);

            // Return a slice of the read buffer containing the data read.
            Ok(&read_buffer[..data_slice.len()])
        }

        async fn write_256b<'b>(
            &mut self,
            _: u32,
            write_buffer: &'b mut [u8],
        ) -> Result<(), Self::Error> {
            if self.data.len() + write_buffer.len() > self.data.capacity() {
                Err(MockFlashError)
            } else {
                self.data.extend_from_slice(write_buffer);
                Ok(())
            }
        }
    }

    // A function to setup and lock a new MockFlash instance.
    fn setup_flash(size: usize) -> VLFS<MockFlash, MockCrc> {
        let flash_memory = MockFlash {
            data: vec![0; size],
        };
        let dummy_crc = MockCrc;

        let vlfs = VLFS::new(flash_memory, dummy_crc);
        let mutex_flash = Mutex::new(vlfs);
        mutex_flash.lock()
    }

    //Erase tests
    //Normal tests
    #[futures_test::test]
    async fn normal_erase_sector_test() {
        let mut flash = setup_flash(4096).await;
        let result = flash.erase_sector_4kib(0).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn normal_erase_block_32kib_test() {
        let mut flash = setup_flash(32768).await;
        let result = flash.erase_block_32kib(0).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn normal_erase_block_64kib_test() {
        let mut flash = setup_flash(65536).await;
        let result = flash.erase_block_64kib(0).await;
        assert!(result.is_ok());
    }

    //Error tests
    #[futures_test::test]
    async fn test_erase_sector_beyond_flash() {
        let mut flash = setup_flash(4096).await;
        let result = flash.erase_sector_4kib(5000).await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_erase_block_32kib_beyond_flash() {
        let mut flash = setup_flash(32768).await;
        let result = flash.erase_block_32kib(50000).await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_erase_block_64kib_beyond_flash() {
        let mut flash = setup_flash(65536).await;
        let result = flash.erase_block_64kib(100000).await;
        assert!(result.is_err());
    }

    //Boundary Test Cases
    //1. address is 0
    //2. address is 4096, 32768, and 65536
    #[futures_test::test]
    async fn test_erase_sector_address_is_0() {
        let mut flash = setup_flash(4096).await;
        let result = flash.erase_sector_4kib(0).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_erase_block_32kib_address_is_0() {
        let mut flash = setup_flash(32768).await;
        let result = flash.erase_block_32kib(0).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_erase_block_64kib_address_is_0() {
        let mut flash = setup_flash(65536).await;
        let result = flash.erase_block_64kib(0).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_erase_sector_address_is_4096() {
        let mut flash = setup_flash(8192).await;
        let result = flash.erase_sector_4kib(4096).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_erase_block_32kib_address_is_32768() {
        let mut flash = setup_flash(65536).await;
        let result = flash.erase_block_32kib(32768).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_erase_block_64kib_address_is_65536() {
        let mut flash = setup_flash(131072).await;
        let result = flash.erase_block_64kib(65536).await;
        assert!(result.is_ok());
    }

    //Reading test functions
    //Normal test
    #[futures_test::test]
    async fn normal_read_test() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(0, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_ok());
        // Check that the read buffer contains the expected data.
        assert_eq!(read_buffer, vec![0u8; 4101]);
    }

    //Error tests
    #[futures_test::test]
    async fn test_read_beyond_4096() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(0, read_buffer.len() + 10, &mut read_buffer)
            .await;
        assert!(result.is_err());
    }
    #[futures_test::test]
    async fn test_read_address_beyond_flash() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(4097, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_read_buffer_not_large_enough() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096 - 5]; // Create a read buffer smaller than read_length
        let result = flash
            .read_4kib(0, read_buffer.len(), &mut read_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_read_uninitialized_flash() {
        let max_size = 4096;
        let flash_memory = MockFlash { data: vec![] }; // Create MockFlash with no data
        let mutex_flash = Mutex::new(flash_memory);
        let mut flash = mutex_flash.lock().await;

        let mut read_buffer = vec![0u8; max_size];
        let result = flash
            .read_4kib(0, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_err());
    }

    //Boundary Test Cases
    //1. read_length is 0
    //2. read_length is 4096
    //4. address is 4096
    //5. read_buffer is 5 bytes larger than read_length
    // address at 0 is not tested since the address is 0 for most of the tests
    #[futures_test::test]
    async fn test_read_length_is_0() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash.read_4kib(0, 0, &mut read_buffer).await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_read_length_is_4096() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(0, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_read_address_is_4096() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(4096, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_read_buffer_is_5_bytes_larger_than_read_length() {
        let mut flash = setup_flash(4096).await;
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(0, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_ok());
    }

    // Test that reading from an erased flash returns Ok and 0xFF for the data read.
    #[futures_test::test]
    async fn test_read_erased_flash() {
        let mut flash = setup_flash(4096).await;
        flash.data.fill(0xFF);
        let mut read_buffer = vec![0u8; 4096];
        let result = flash
            .read_4kib(0, read_buffer.len() - 5, &mut read_buffer)
            .await;
        assert!(result.is_ok());
        assert_eq!(read_buffer, vec![0xFFu8; 4096]);
    }

    //Writing test functions

    //Normal test
    #[futures_test::test]
    async fn normal_write_test() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 4096];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_ok());
        // Optionally, check that the write buffer contains the expected data.
        assert_eq!(write_buffer, vec![1u8; 4096]);
    }

    //Error tests
    // 1. Writing more data than the flash memory's capacity
    // 2. Trying to write to an address that is not 256-byte aligned
    // 3. Attempting to write data when the write buffer is empty.
    // 4. Writing data with a size that is not a multiple of 256
    // 5. Trying to write data to a memory that is not initialized or ready.
    // 6. Writing data when the write buffer length is less than write_length + 5.

    #[futures_test::test]
    async fn test_write_disk_capacity() {
        let mut flash = setup_flash(16384 * 4096).await;
        let mut write_buffer = vec![1u8; 16384 * 4096 + 1]; // Create a write buffer larger than the MockFlash's capacity
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_write_address_not_256_byte_aligned() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 4096];
        let result = flash
            .write(1, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_write_empty_buffer() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_write_length_not_multiple_of_256() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 255];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_write_uninitialized_flash() {
        let flash_memory = MockFlash { data: vec![] }; // Create MockFlash with no data
        let mutex_flash = Mutex::new(flash_memory);
        let mut flash = mutex_flash.lock().await;
        let mut write_buffer = vec![1u8; 4096];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_write_buffer_too_small() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 4096 - 5];
        let result = flash.write(0, write_buffer.len(), &mut write_buffer).await;
        assert!(result.is_err());
    }

    //Boundary Test Cases
    // Writing data of size exactly equal to the flash memory's capacity.
    // Writing data to the last 256-byte-aligned address in the memory.
    // Writing exactly 256 bytes of data (the minimum allowed by the function).

    #[futures_test::test]
    async fn test_write_disk_capacity_boundary() {
        let mut flash = setup_flash(16384 * 4096).await;
        let mut write_buffer = vec![1u8; 16384 * 4096];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_write_last_256_byte_aligned_address() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 4096];
        let result = flash.write(4096 - 256, 256, &mut write_buffer).await;
        assert!(result.is_ok());
    }

    #[futures_test::test]
    async fn test_write_256_bytes() {
        let mut flash = setup_flash(4096).await;
        let mut write_buffer = vec![1u8; 256];
        let result = flash
            .write(0, write_buffer.len() - 5, &mut write_buffer)
            .await;
        assert!(result.is_ok());
    }

    //Testing file rw
    #[futures_test::test]
    async fn test_write_and_read_files() {
        let mut flash = setup_flash(4096).await;
        for file_length in 1..=4096 {
            let write_buffer = (0..file_length).map(|i| i as u8).collect::<Vec<_>>();
            let write_result = flash.write_256b(0, &mut write_buffer.clone()).await;
            assert!(
                write_result.is_ok(),
                "Failed to write file of length {}",
                file_length
            );

            let mut read_buffer = vec![0u8; file_length];
            let read_result = flash.read_4kib(0, file_length, &mut read_buffer).await;
            assert!(
                read_result.is_ok(),
                "Failed to read file of length {}",
                file_length
            );

            assert_eq!(
                read_buffer, write_buffer,
                "Data mismatch for file of length {}",
                file_length
            );
        }
    }
}
