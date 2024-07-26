use core::mem::transmute;

use bitvec::prelude::*;

use super::SerializeBitOrder;

pub trait BitSliceWritable {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize;
}

impl BitSliceWritable for bool {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        slice.set(0, self);
        1
    }
}

impl BitSliceWritable for u8 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = [self];
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..8]).copy_from_bitslice(data);
        8
    }
}

impl BitSliceWritable for f32 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..32]).copy_from_bitslice(data);
        32
    }
}

impl BitSliceWritable for f64 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) -> usize {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..64]).copy_from_bitslice(data);
        64
    }
}

pub trait FromBitSlice: Sized {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self;

    fn len_bits() -> usize;
}

impl FromBitSlice for bool {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        slice[0]
    }

    fn len_bits() -> usize {
        1
    }
}

impl FromBitSlice for u8 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..8];
        slice.load_le::<u8>()
    }
    fn len_bits() -> usize {
        8
    }
}

impl FromBitSlice for f32 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..32];
        unsafe { transmute(slice.load_le::<u32>()) }
    }
    fn len_bits() -> usize {
        32
    }
}

impl FromBitSlice for f64 {
    fn from_bit_slice(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..64];
        unsafe { transmute(slice.load_le::<u64>()) }
    }

    fn len_bits() -> usize {
        64
    }
}
