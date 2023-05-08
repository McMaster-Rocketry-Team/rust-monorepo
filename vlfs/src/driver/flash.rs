//This file will be explained using comments, so it will be easier to understand the code.

// Importing the Deref and DerefMut traits from the core library.
use core::ops::{Deref, DerefMut};
// Importing the RawMutex and MutexGuard traits from the embassy_sync library.
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};

// This trait will be implemented by the flash driver.
pub trait Flash {
    // This is the error type that will be returned by the flash driver.
    type Error: defmt::Format;

    //This function returns the size of the flash memory in bytes.
    fn size(&self) -> u32;

    //This function resets the flash memory by setting all the bits to 1.
    async fn reset(&mut self) -> Result<(), Self::Error>;

    // erase methods must set all the erased bits to 1 - VLFS relies on this assumption
    //These functions return no useful value if they succeed, and an error if they fail.
    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error>;
    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error>;


    async fn read_4kib<'b>(
        /*
        Purpose: read 4KiB of data from the memory device starting at the specified address
        Parameters:
            &mut self:     a reference to the memory device object
            address:       u32 integer that specifies the starting address of the 4KiB block that will be read
            read_length:   usize integer that specifies the # of bytes to be read from the memory device. 
            read_buffer:   buffer where read data will be stored
        
        Outputs:
            Ok(&'b [u8]):  returns a reference to the slice of bytes that were read
            Err(Self::Error): returns error that occured
        
        Note:
            maximum read length is 4 kb
            size of the buffer must be at least 5 bytes larger than read_length */
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error>;//


    async fn write_256b<'b>(
        /*
        Purpose: write 256 bytes of data to the memory device starting at the specified address
        Parameters:
            &mut self:     a reference to the memory device object
            address:       u32 integer that specifies the starting address of the 256 byte block that will be written
            write_buffer:  buffer where data to be written is stored
        
        Outputs:
            Ok(()):        returns nothing if the write operation was successful
            Err(Self::Error): returns error that occured

        Note:
            size of the buffer must be at least 261 bytes long */

        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error>;

    //read arbitrary length, length of read_buffer must be larger or equal to read_length + 5
    //refer to the read_4kib function for more details on the parameters and outputs as they are the same
    async fn read<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        /*
        This loop reads the data from the flash by calling read_4kib multiple times, with each call
        reading up to 4096 bytes at a time, until it has read the entire requested length.

        Variables:
            bytes_read: usize integer that keeps track of the number of bytes that have been read so far
            length:     usize integer that specifies the number of bytes to be read in the current iteration
         */

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
        // from the 5th byte to the 5th byte + the read_length. The first 5 bytes are skipped because they are used to store the
        // address and length of the data that was read.
        Ok(&read_buffer[5..(5 + read_length)])
    }

    // write arbitary length (must be a multiple of 256 bytes)
    // address must be 256-byte-aligned
    // length of write_buffer must be larger or equal to write_length + 5
    // refer to the write_256b function for more details on the parameters and outputs as they are the same
    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        /*
        This loop writes the data to the flash by calling write_256b multiple times, with each call
        writing up to 256 bytes at a time, until it has written the entire requested length.

        Variables:
            bytes_written: usize integer that keeps track of the number of bytes that have been written so far
            length:        usize integer that specifies the number of bytes to be written in the current iteration
         */
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
            //Update the number of bytes written.
            bytes_written += length;
        }
        // Return Ok if the write operation was successful.
        Ok(())
    }
}

// `MutexGuard` is a type from the `lock_api` crate that represents a held lock on a mutex
// This implementation of `Flash` is for a flash memory that can be accessed through a mutex
impl<'a, M, T> Flash for MutexGuard<'a, M, T>
where
    M: RawMutex, // M is a mutex type from the `lock_api` crate that implements the `RawMutex` trait. 
                 //To put into simpler terms, `M` is a mutex type that implements the `lock` and `unlock` methods
    T: Flash,    // T is a type that implements the `Flash` trait. This is the type of the wrapped object
{

    // `Error` is the error type of the wrapped `T` object
    type Error = T::Error;

    // `size` returns the total size of the flash memory, in bytes
    fn size(&self) -> u32 {
        self.deref().size() // Call the `size` method on the wrapped `T` object
    }

    // `reset` erases all contents of the flash memory
    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.deref_mut().reset().await // Call the `reset` method on the wrapped `T` object
    }

    // `erase_sector_4kib` erases a 4KB sector of the flash memory starting at the given address
    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_sector_4kib(address).await // Call the `erase_sector_4kib` method on the wrapped `T` object
    }

    // `erase_block_32kib` erases a 32KB block of the flash memory starting at the given address
    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_32kib(address).await // Call the `erase_block_32kib` method on the wrapped `T` object
    }

    // `erase_block_64kib` erases a 64KB block of the flash memory starting at the given address
    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.deref_mut().erase_block_64kib(address).await // Call the `erase_block_64kib` method on the wrapped `T` object
    }

    // `read_4kib` reads a 4KB block of the flash memory starting at the given address into the provided buffer
    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        // Call the `read_4kib` method on the wrapped `T` object
        self.deref_mut()
            .read_4kib(address, read_length, read_buffer)
            .await // Call the `read_4kib` method on the wrapped `T` object
    }
    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        // Call the `write_256b` method on the wrapped `T` object
        self.deref_mut().write_256b(address, write_buffer).await
    }
}
