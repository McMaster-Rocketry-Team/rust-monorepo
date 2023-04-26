use crate::driver::crc::Crc;

use super::io_traits::Writer;

pub struct WriteBuffer<'a, const N: usize> {
    buffer: &'a mut [u8; N],
    start_offset: usize,
    offset: usize,
}

impl<'a, const N: usize> WriteBuffer<'a, N> {
    pub fn new(buffer: &'a mut [u8; N], start_offset: usize) -> Self {
        Self {
            buffer,
            start_offset,
            offset: start_offset,
        }
    }

    pub fn len(&self) -> usize {
        self.offset - self.start_offset
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len() - self.start_offset
    }

    pub fn reset(&mut self) {
        self.set_offset(0);
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset + self.start_offset;
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.offset + slice.len() > self.buffer.len() {
            return;
        }
        self.buffer[self.offset..(self.offset + slice.len())].copy_from_slice(slice);
        self.offset += slice.len();
    }

    pub fn replace_u16(&mut self, i: usize, value: u16) {
        self.buffer[(self.start_offset + i)..(self.start_offset + i + 2)]
            .copy_from_slice(&value.to_be_bytes());
    }

    pub fn replace_u32(&mut self, i: usize, value: u32) {
        self.buffer[(self.start_offset + i)..(self.start_offset + i + 4)]
            .copy_from_slice(&value.to_be_bytes());
    }

    pub fn align_4_bytes(&mut self) {
        while (self.offset - self.start_offset) % 4 != 0 {
            self.extend_from_u8(0xFF);
        }
    }

    pub fn calculate_crc<C: Crc>(&self, crc: &mut C) -> u32 {
        crc.calculate(self.as_slice_without_start())
    }

    pub fn as_slice_without_start(&self) -> &[u8] {
        &self.buffer[self.start_offset..self.offset]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer
    }
}


impl<'a, const N: usize> Writer for WriteBuffer<'a, N> {
    fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.offset + slice.len() > self.buffer.len() {
            return;
        }
        self.buffer[self.offset..(self.offset + slice.len())].copy_from_slice(slice);
        self.offset += slice.len();
    }

    fn extend_from_u8(&mut self, value: u8) {
        self.buffer[self.offset] = value;
        self.offset += 1;
    }
}

#[macro_export]
macro_rules! new_write_buffer {
    ($name: ident, $length: expr) => {
        new_write_buffer!($name, $length, 5);
    };
    ($name: ident, $length: expr, $start_offset: expr) => {
        let mut $name: [u8; { $length + $start_offset }] = [0u8; $length + $start_offset];
        let mut $name: WriteBuffer<{ $length + $start_offset }> =
            WriteBuffer::new(&mut $name, $start_offset);
    };
}
