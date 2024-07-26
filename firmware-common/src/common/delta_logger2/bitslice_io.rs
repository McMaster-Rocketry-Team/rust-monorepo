use super::{
    bitvec_serialize_traits::{BitSliceWritable, FromBitSlice},
    SerializeBitOrder,
};
use bitvec::prelude::*;
use core::future::Future;

pub struct BitSliceWriter<const N: usize> {
    len: usize,
    data: BitArray<[u8; N], SerializeBitOrder>,
}

impl<const N: usize> BitSliceWriter<N> {
    pub fn write(&mut self, value: impl BitSliceWritable) {
        let len = value.write(&mut self.data[self.len..]);
        self.len += len;
    }

    /// returns a slice of the full bytes
    /// e.g. if the length is 20 bits, this will return the first 2 bytes
    pub fn view_full_byte_slice(&self) -> &[u8] {
        let full_bytes = self.len / 8;
        &self.data.as_raw_slice()[..full_bytes]
    }

    /// returns a slice of the data, including bytes that are not fully filled
    /// e.g. if the length is 20 bits, this will return the first 3 bytes
    pub fn view_all_data_slice(&self) -> &[u8] {
        let bytes = (self.len + 7) / 8;
        &self.data.as_raw_slice()[..bytes]
    }

    /// removes the full bytes from the slice
    /// e.g. if the length is 20 bits, this will remove the first 2 bytes
    /// and the new length will be 4 bits
    pub fn clear_full_byte_slice(&mut self) {
        let full_bytes = self.len / 8;
        self.len = self.len % 8;
        let raw_slice = self.data.as_raw_mut_slice();
        raw_slice[0] = raw_slice[full_bytes];
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<const N: usize> Default for BitSliceWriter<N> {
    fn default() -> Self {
        Self {
            len: 0,
            data: Default::default(),
        }
    }
}

pub trait BitArraySerializable {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>);
}

pub struct BitSliceReader<const N: usize> {
    buffer: BitArray<[u8; N], SerializeBitOrder>,
    start: usize,
    end: usize,
}

impl<const N: usize> Default for BitSliceReader<N> {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            start: 0,
            end: 0,
        }
    }
}

impl<const N: usize> BitSliceReader<N> {
    pub(super) fn len_bits(&self) -> usize {
        self.end - self.start
    }

    pub(super) fn free_bytes(&self) -> usize {
        (self.buffer.len() - self.len_bits()) / 8
    }

    /// move existing data to the beginning of the buffer and append new data
    pub(super) fn replenish_bytes(&mut self, data: &[u8]) {
        let len_bits = self.len_bits();
        let dest = (8 - len_bits % 8) % 8;
        self.buffer.copy_within(self.start..self.end, dest);
        self.start = dest;
        self.end = dest + len_bits;

        let new_start_byte_i = self.end / 8;
        (&mut self.buffer.as_raw_mut_slice()[new_start_byte_i..(new_start_byte_i + data.len())])
            .copy_from_slice(data);
        self.end += data.len() * 8;
    }

    pub(super) async fn replenish_bytes_async_mut<'a, 'b, FN, F, E>(
        &'a mut self,
        f: FN,
    ) -> Result<usize, E>
    where
        FN: FnOnce(&'b mut [u8]) -> F,
        F: Future<Output = Result<usize, E>>,
        'a: 'b,
    {
        let len_bits = self.len_bits();
        if len_bits == 0 {
            self.start = 0;
            self.end = 0;
        } else {
            let dest = (8 - len_bits % 8) % 8;
            self.buffer.copy_within(self.start..self.end, dest);
            self.start = dest;
            self.end = dest + len_bits;
        }

        let new_start_byte_i = self.end / 8;
        let buffer = &mut self.buffer.as_raw_mut_slice()[new_start_byte_i..];
        let read_len_bytes = f(buffer).await?;
        self.end += read_len_bytes * 8;
        Ok(read_len_bytes)
    }

    pub fn read<T: FromBitSlice>(&mut self) -> Option<T> {
        if self.len_bits() < T::len_bits() {
            return None;
        }
        let data = T::from_bit_slice(&self.buffer[self.start..self.end]);
        self.start += T::len_bits();
        Some(data)
    }

    pub fn skip_byte(&mut self) {
        self.start = (self.start + 7) / 8 * 8;
    }
}

pub trait BitArrayDeserializable {
    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self;

    fn len_bits() -> usize;
}

#[cfg(test)]
mod test {
    use core::convert::Infallible;

    use super::*;

    #[test]
    fn test_bit_slice_reader() {
        let mut reader: BitSliceReader<4> = Default::default();
        assert_eq!(reader.len_bits(), 0);
        assert_eq!(reader.free_bytes(), 4);

        reader.replenish_bytes(&[0xFF, 0b11111110]);
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<bool>(), Some(true));
        assert_eq!(reader.len_bits(), 15);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<u8>(), Some(0b01111111));
        assert_eq!(reader.read::<u8>(), None);
        assert_eq!(reader.len_bits(), 7);
        assert_eq!(reader.free_bytes(), 3);

        reader.replenish_bytes(&[0xFF; 3]);
        assert_eq!(reader.len_bits(), 7 + 3 * 8);
        assert_eq!(reader.free_bytes(), 0);
        assert_eq!(reader.start, 1);

        for _ in 0..7 {
            reader.read::<bool>();
        }

        assert_eq!(reader.len_bits(), 3 * 8);
        assert_eq!(reader.free_bytes(), 1);
        reader.replenish_bytes(&[0xFF]);
        assert_eq!(reader.len_bits(), 4 * 8);
        assert_eq!(reader.free_bytes(), 0);
    }

    #[tokio::test]
    async fn test_bit_slice_reader_async_replenish() {
        let mut reader: BitSliceReader<4> = Default::default();
        assert_eq!(reader.len_bits(), 0);
        assert_eq!(reader.free_bytes(), 4);

        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                assert_eq!(buffer.len(), 4);
                buffer[0] = 0xFF;
                buffer[1] = 0b11111110;
                Ok(2)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<bool>(), Some(true));
        assert_eq!(reader.len_bits(), 15);
        assert_eq!(reader.free_bytes(), 2);

        assert_eq!(reader.read::<u8>(), Some(0b01111111));
        assert_eq!(reader.read::<u8>(), None);
        assert_eq!(reader.len_bits(), 7);
        assert_eq!(reader.free_bytes(), 3);

        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                buffer[0] = 0xFF;
                buffer[1] = 0xFF;
                buffer[2] = 0xFF;
                Ok(3)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 7 + 3 * 8);
        assert_eq!(reader.free_bytes(), 0);
        assert_eq!(reader.start, 1);

        for _ in 0..7 {
            reader.read::<bool>();
        }

        assert_eq!(reader.len_bits(), 3 * 8);
        assert_eq!(reader.free_bytes(), 1);
        reader
            .replenish_bytes_async_mut::<_, _, Infallible>(async |buffer: &mut [u8]| {
                buffer[0] = 0xFF;
                Ok(1)
            })
            .await
            .unwrap();
        assert_eq!(reader.len_bits(), 4 * 8);
        assert_eq!(reader.free_bytes(), 0);
    }

    #[test]
    fn test_bit_slice_reader_skip_byte() {
        let mut reader: BitSliceReader<4> = Default::default();
        reader.replenish_bytes(&[0xFF, 0b11111110]);

        reader.skip_byte();
        assert_eq!(reader.len_bits(), 16);
        assert_eq!(reader.read::<bool>(), Some(true));

        reader.skip_byte();
        assert_eq!(reader.len_bits(), 8);
        assert_eq!(reader.read::<u8>(), Some(0b11111110));
    }
}
