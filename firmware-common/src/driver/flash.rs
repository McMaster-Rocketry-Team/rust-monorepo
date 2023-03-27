use defmt::*;

pub trait SpiFlash {
    // size in bytes
    fn size(&self) -> u32;

    async fn erase_sector_4kib(&mut self, address: u32);
    async fn erase_block_32kib(&mut self, address: u32);
    async fn erase_block_64kib(&mut self, address: u32);

    // maximum read length is 4 kb
    // size of the buffer must be at least 5 bytes larger than read_length
    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> &'b [u8];

    // Write a full page of 256 bytes, the last byte of the address is ignored
    // The write buffer must be at least 261 bytes long, where the last 256 bytes are the data to write
    async fn write_256b<'b>(&mut self, address: u32, write_buffer: &'b mut [u8]);

    // read arbitary length, N must be larger or equal to read_length + 5
    async fn read<'b, 'c, const N: usize>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut ReadBuffer<'c, N>,
    ) {
        // info!("hello");
        // let read_buffer = read_buffer.as_mut_slice();
        // info!(
        //     "read read_buffer: {}, {}",
        //     read_buffer.len(),
        //     &read_buffer[0..10]
        // );
        // let mut bytes_read = 0;
        // while bytes_read < read_length {
        //     let length = if read_length - bytes_read > 4096 {
        //         4096
        //     } else {
        //         read_length - bytes_read
        //     };
        //     info!("reading {}/{} bytes", bytes_read, read_length);
        //     // self.read_4kib(
        //     //     address + bytes_read as u32,
        //     //     length,
        //     //     &mut read_buffer[bytes_read..],
        //     // )
        //     // .await;
        //     bytes_read += length;
        // }
    }

    // write arbitary length (must be a multiple of 256 bytes)
    // address must be 256-byte-aligned
    // N must be larger or equal to read_length + 5
    async fn write<'b, 'c, const N: usize>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut WriteBuffer<'c, N>,
    ) {
        let write_buffer = write_buffer.as_mut_slice();
        let mut bytes_written = 0;
        while bytes_written < write_length {
            let length = if write_length - bytes_written > 256 {
                256
            } else {
                write_length - bytes_written
            };
            info!("writing {}/{} bytes", bytes_written, write_length);
            self.write_256b(
                address + bytes_written as u32,
                &mut write_buffer[bytes_written..],
            )
            .await;
            bytes_written += length;
        }
    }
}

const START_OFFSET: usize = 5;

pub struct ReadBuffer<'a, const N: usize> {
    buffer: &'a mut [u8; N],
    offset: usize,
}

impl<'a, const N: usize> ReadBuffer<'a, N> {
    pub fn new(buffer: &'a mut [u8; N]) -> Self {
        Self {
            buffer,
            offset: START_OFFSET,
        }
    }

    pub fn len(&self) -> usize {
        N - START_OFFSET
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

    pub fn read_u16(&mut self) -> u16 {
        if 2 > self.buffer.len() - self.offset {
            return 0;
        }
        let value = u16::from_be_bytes(
            self.buffer[self.offset..(self.offset + 2)]
                .try_into()
                .unwrap(),
        );
        self.offset += 2;
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

    pub fn as_slice_without_start(&mut self) -> &[u8] {
        &self.buffer[START_OFFSET..]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer
    }
}

#[macro_export]
macro_rules! new_read_buffer {
    ($name: ident, $length:expr) => {
        let mut $name: [u8; {$length + 5}] = [0u8; $length + 5];
        let mut $name: ReadBuffer<{$length + 5}> = ReadBuffer::new(&mut $name);
    };
}

pub struct WriteBuffer<'a, const N: usize> {
    buffer: &'a mut [u8; N],
    offset: usize,
}

impl<'a, const N: usize> WriteBuffer<'a, N> {
    pub fn new(buffer: &'a mut [u8; N]) -> Self {
        Self {
            buffer,
            offset: START_OFFSET,
        }
    }

    pub fn len(&self) -> usize {
        N - START_OFFSET
    }

    pub fn reset(&mut self) {
        self.offset = START_OFFSET;
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.offset + slice.len() > self.buffer.len() {
            return;
        }
        self.buffer[self.offset..(self.offset + slice.len())].copy_from_slice(slice);
        self.offset += slice.len();
    }

    pub fn extend_from_u8(&mut self, value: u8) {
        self.buffer[self.offset] = value;
        self.offset += 1;
    }

    pub fn extend_from_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_be_bytes());
    }

    pub fn extend_from_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_be_bytes());
    }

    pub fn replace_u32(&mut self, value: u32, i: usize) {
        self.buffer[i..(i + 4)].copy_from_slice(&value.to_be_bytes());
    }

    pub fn align_4_bytes(&mut self) {
        while (self.offset - 5) % 4 != 0 {
            self.offset += 1;
        }
    }

    pub fn as_slice_without_start(&mut self) -> &[u8] {
        &self.buffer[START_OFFSET..]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer
    }
}

#[macro_export]
macro_rules! new_write_buffer {
    ($name: ident, $length:expr) => {
        let mut $name: [u8; {$length + 5}] = [0u8; $length + 5];
        let mut $name: WriteBuffer<{$length + 5}> = WriteBuffer::new(&mut $name);
    };
}
