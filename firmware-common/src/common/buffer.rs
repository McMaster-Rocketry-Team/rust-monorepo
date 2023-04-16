pub struct WriteBuffer<'a, const N: usize> {
    buffer: &'a mut [u8; N],
    offset: usize,
}

const START_OFFSET: usize = 5;

impl<'a, const N: usize> WriteBuffer<'a, N> {
    pub fn new(buffer: &'a mut [u8; N]) -> Self {
        Self {
            buffer,
            offset: START_OFFSET,
        }
    }

    pub fn len(&self) -> usize {
        self.offset - START_OFFSET
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len() - START_OFFSET
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
        &self.buffer[START_OFFSET..self.offset]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer
    }
}

#[macro_export]
macro_rules! new_write_buffer {
    ($name: ident, $length:expr) => {
        let mut $name: [u8; { $length + 5 }] = [0u8; $length + 5];
        let mut $name: WriteBuffer<{ $length + 5 }> = WriteBuffer::new(&mut $name);
    };
}