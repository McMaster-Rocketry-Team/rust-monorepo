pub trait SpiFlash {
    // size in bytes
    fn size(&self) -> u32;

    async fn erase_sector_4kb(&mut self, address: u32);
    async fn erase_block_32kb(&mut self, address: u32);
    async fn erase_block_64kb(&mut self, address: u32);

    // maximum read length is 4 kb
    // size of the buffer must be at least 5 bytes larger than read_length
    async fn read_4kb<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> &'b [u8];

    // read arbitary length, read_buffer must be larger or equal to read_length
    async fn read<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> &'b [u8];

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be 261 bytes long, where the last 256 bytes are the data to write
    async fn write_page<'b>(&mut self, address: u32, write_buffer: &'b mut WriteBuffer);

    // returns a buffer of 264 bytes,
    // where the first 3 bytes are the padding for 4-byte aligmnent (required by rkyv)
    // The following 5 bytes are the command and the address
    // the last 256 bytes are the data
    async fn read_256_bytes(&mut self, address: u32) -> ReadBuffer {
        let mut buffer = [0u8; 264];
        self.read_4kb(address, 256, &mut buffer[3..]).await;
        ReadBuffer::new(buffer)
    }

    // returns a buffer of 4104 bytes,
    // where the first 3 bytes are the padding for 4-byte aligmnent (required by rkyv)
    // The following 5 bytes are the command and the address
    // the last 4096 bytes are the data
    async fn read_sector_4kb(&mut self, address: u32) -> [u8; 4104] {
        let mut buffer = [0u8; 4104];
        self.read_4kb(address, 4096, &mut buffer[3..]).await;
        buffer
    }

    
}

pub struct ReadBuffer {
    buffer: [u8; 264],
    offset: usize,
}

impl ReadBuffer {
    pub fn new(buffer: [u8; 264]) -> Self {
        Self { buffer, offset: 8 }
    }

    pub fn read_u32(&mut self) -> u32 {
        if 4 > self.buffer.len() - self.offset {
            return 0;
        }
        let value = u32::from_be_bytes(
            self.buffer[self.offset..(self.offset + 4)]
                .try_into()
                .unwrap(),
        );
        self.offset += 4;
        value
    }

    pub fn read_slice(&mut self, length: usize) -> &[u8] {
        if length > self.buffer.len() - self.offset {
            return &[];
        }
        let slice = &self.buffer[self.offset..(self.offset + length)];
        self.offset += length;
        slice
    }

    pub fn align_4_bytes(&mut self) {
        while self.offset % 4 != 0 {
            self.offset += 1;
        }
    }
}

pub struct WriteBuffer {
    buffer: [u8; 261],
    offset: usize,
}

impl WriteBuffer {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; 261],
            offset: 5,
        }
    }

    pub fn reset(&mut self) {
        self.offset = 5;
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.offset + slice.len() > self.buffer.len() {
            return;
        }
        self.buffer[self.offset..(self.offset + slice.len())].copy_from_slice(slice);
        self.offset += slice.len();
    }

    pub fn extend_from_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_be_bytes());
    }

    pub fn align_4_bytes(&mut self) {
        while (self.offset - 5) % 4 != 0 {
            self.offset += 1;
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8; 261] {
        &mut self.buffer
    }
}
