use core::mem::transmute;

use bitvec::prelude::*;

use super::SerializeBitOrder;

pub trait BitSliceRWable {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>);

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self;

    fn len_bits() -> usize;
}

impl BitSliceRWable for bool {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        slice.set(0, self);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        slice[0]
    }

    fn len_bits() -> usize {
        1
    }
}

impl BitSliceRWable for u8 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        let data = [self];
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..8]).copy_from_bitslice(data);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..8];
        slice.load_le::<u8>()
    }

    fn len_bits() -> usize {
        8
    }
}

impl BitSliceRWable for i64 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..8]).copy_from_bitslice(data);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..8];
        slice.load_le::<i64>()
    }

    fn len_bits() -> usize {
        64
    }
}

impl BitSliceRWable for f32 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..32]).copy_from_bitslice(data);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..32];
        unsafe { transmute(slice.load_le::<u32>()) }
    }

    fn len_bits() -> usize {
        32
    }
}

impl BitSliceRWable for f64 {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        let data = self.to_le_bytes();
        let data: &BitSlice<u8, SerializeBitOrder> = data.view_bits();
        (&mut slice[..64]).copy_from_bitslice(data);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        let slice = &slice[..64];
        unsafe { transmute(slice.load_le::<u64>()) }
    }

    fn len_bits() -> usize {
        64
    }
}

impl<T:BitSliceRWable> BitSliceRWable for (T, T) {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        self.0.write(&mut slice[..T::len_bits()]);
        self.1.write(&mut slice[T::len_bits()..]);
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        (
            T::read(&slice[..T::len_bits()]),
            T::read(&slice[T::len_bits()..]),
        )
    }

    fn len_bits() -> usize {
        64 + 64
    }
}

impl<T: BitSliceRWable> BitSliceRWable for Option<T> {
    fn write(self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
        if let Some(value) = self {
            slice.set(0, true);
            value.write(&mut slice[1..]);
        } else {
            slice.set(0, false);
            (&mut slice[1..(T::len_bits() + 1)]).fill(true);
        }
    }

    fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
        if slice[0] {
            Some(T::read(&slice[1..]))
        } else {
            None
        }
    }

    fn len_bits() -> usize {
        T::len_bits() + 1
    }
}
