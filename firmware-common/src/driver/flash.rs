pub trait SpiFlash {
    // size in bytes
    fn size(&self) -> u32;

    async fn erase_sector_4kib(&mut self, address: u32);
    async fn erase_block_32kib(&mut self, address: u32);
    async fn erase_block_64kib(&mut self, address: u32);

    // read arbitary length, N must be larger or equal to read_length
    async fn read<'b, const N: usize>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut ReadBuffer<N, 5>,
    ) where
        [u8; N + 5]: Sized;

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be 261 bytes long, where the last 256 bytes are the data to write
    async fn write_page<'b>(&mut self, address: u32, write_buffer: &'b mut WriteBuffer);
}

pub struct ReadBuffer<const N: usize, const START_OFFSET: usize>
where
    [u8; N + START_OFFSET]: Sized,
{
    buffer: [u8; N + START_OFFSET],
    offset: usize,
}

impl<const N: usize, const START_OFFSET: usize> ReadBuffer<N, START_OFFSET>
where
    [u8; N + START_OFFSET]: Sized,
{
    pub fn new() -> Self {
        Self {
            buffer: [0u8; N + START_OFFSET],
            offset: START_OFFSET,
        }
    }

    pub fn len(&self) -> usize {
        N
    }

    pub fn reset(&mut self) {
        self.offset = START_OFFSET;
    }

    pub fn read_u8(&mut self) -> u8 {
        if 1 > self.buffer.len() - self.offset {
            return 0;
        }
        let value = self.buffer[self.offset];
        self.offset += 1;
        value
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

    pub fn as_mut_slice(&mut self) -> &mut [u8; N + START_OFFSET] {
        &mut self.buffer
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
