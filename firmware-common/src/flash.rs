pub trait SpiFlash {
    async fn erase_sector_4kb(&mut self, address: u32);
    async fn erase_block_64kb(&mut self, address: u32);

    // maximum read length is 256 bytes
    // size of the buffer must be at least 5 bytes larger than read_length
    async fn read<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> &'b [u8];

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be 261 bytes long, where the last 256 bytes are the data to write
    async fn write_page<'b>(&mut self, address: u32, write_buffer: &'b mut [u8; 261]);
}
