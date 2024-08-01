use core::fmt::Debug;

use crate::common::delta_logger::SerializeBitOrder;
use bitvec::prelude::*;
use packed_struct::prelude::*;
use rkyv::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Fallible, Resolver, Serialize,
};

use super::delta_logger::bitslice_primitive::BitSlicePrimitive;

pub trait VariableIntTrait {
    type Base;
    type Packed: BitSlicePrimitive + Debug + Clone;
}

pub struct VariableInt<const BITS: usize>;

pub struct VariableIntRkyvWrapper;

macro_rules! impl_variable_int {
    ($base_type: ty, $bits:literal) => {
        impl VariableIntTrait for VariableInt<$bits> {
            type Base = $base_type;
            type Packed = Integer<$base_type, packed_bits::Bits<$bits>>;
        }

        impl BitSlicePrimitive for Integer<$base_type, packed_bits::Bits<$bits>> {
            fn write(&self, slice: &mut BitSlice<u8, SerializeBitOrder>) {
                let bits = self.view_bits::<SerializeBitOrder>();
                let bits = unsafe { bits.align_to::<u8>().1 };
                (&mut slice[..$bits]).copy_from_bitslice(&bits[..$bits]);
            }

            fn read(slice: &BitSlice<u8, SerializeBitOrder>) -> Self {
                slice[0..$bits].load_le::<$base_type>().into()
            }

            fn len_bits() -> usize {
                $bits
            }
        }

        impl<S: Fallible + ?Sized> SerializeWith<Integer<$base_type, packed_bits::Bits<$bits>>, S>
            for VariableIntRkyvWrapper
        {
            fn serialize_with(
                field: &Integer<$base_type, packed_bits::Bits<$bits>>,
                serializer: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                let field: $base_type = (*field).into();
                field.serialize(serializer)
            }
        }

        impl ArchiveWith<Integer<$base_type, packed_bits::Bits<$bits>>> for VariableIntRkyvWrapper {
            type Archived = Archived<$base_type>;
            type Resolver = Resolver<$base_type>;

            unsafe fn resolve_with(
                field: &Integer<$base_type, packed_bits::Bits<$bits>>,
                pos: usize,
                _: (),
                out: *mut Self::Archived,
            ) {
                let field: $base_type = (*field).into();
                field.resolve(pos, (), out);
            }
        }

        impl<D: Fallible + ?Sized>
            DeserializeWith<Archived<$base_type>, Integer<$base_type, packed_bits::Bits<$bits>>, D>
            for VariableIntRkyvWrapper
        where
            Archived<$base_type>: Deserialize<$base_type, D>,
        {
            fn deserialize_with(
                field: &Archived<$base_type>,
                deserializer: &mut D,
            ) -> Result<Integer<$base_type, packed_bits::Bits<$bits>>, D::Error> {
                Ok(field.deserialize(deserializer)?.into())
            }
        }
    };
}

impl_variable_int!(u8, 1);
impl_variable_int!(u8, 2);
impl_variable_int!(u8, 3);
impl_variable_int!(u8, 4);
impl_variable_int!(u8, 5);
impl_variable_int!(u8, 6);
impl_variable_int!(u8, 7);
impl_variable_int!(u8, 8);
impl_variable_int!(u16, 9);
impl_variable_int!(u16, 10);
impl_variable_int!(u16, 11);
impl_variable_int!(u16, 12);
impl_variable_int!(u16, 13);
impl_variable_int!(u16, 14);
impl_variable_int!(u16, 15);
impl_variable_int!(u16, 16);
impl_variable_int!(u32, 17);
impl_variable_int!(u32, 18);
impl_variable_int!(u32, 19);
impl_variable_int!(u32, 20);
impl_variable_int!(u32, 21);
impl_variable_int!(u32, 22);
impl_variable_int!(u32, 23);
impl_variable_int!(u32, 24);
impl_variable_int!(u32, 25);
impl_variable_int!(u32, 26);
impl_variable_int!(u32, 27);
impl_variable_int!(u32, 28);
impl_variable_int!(u32, 29);
impl_variable_int!(u32, 30);
impl_variable_int!(u32, 31);
impl_variable_int!(u32, 32);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize() {
        let mut arr = bitarr![u8, Lsb0; 0; 16];
        let num: <VariableInt<10> as VariableIntTrait>::Packed = 0b1011111111.into();
        num.write(arr.as_mut_bitslice());
        for i in 0..16 {
            print!("{}", if arr[i] { 1 } else { 0 });
        }
        println!("");

        let num = <VariableInt<10> as VariableIntTrait>::Packed::read(arr.as_bitslice());
        assert_eq!(num, 0b1011111111.into());
    }

    use rkyv::{
        archived_root,
        ser::{serializers::AllocSerializer, Serializer},
        Archive, Deserialize, Infallible, Serialize,
    };

    #[derive(Archive, Deserialize, Serialize)]
    struct Example {
        #[with(VariableIntRkyvWrapper)]
        a: Integer<u8, packed_bits::Bits<5>>,
        b: i32,
    }

    #[test]
    fn test_var_int_rkyv() {
        let value = Example { a: 4.into(), b: 9 };

        let mut serializer = AllocSerializer::<4096>::default();
        serializer.serialize_value(&value).unwrap();
        let buf = serializer.into_serializer().into_inner();

        let archived = unsafe { archived_root::<Example>(buf.as_ref()) };
        // The wrapped field has been incremented
        assert_eq!(archived.a, 4u8);
        // ... and the unwrapped field has not
        assert_eq!(archived.b, 9);

        let deserialized: Example = archived.deserialize(&mut Infallible).unwrap();
        // The wrapped field is back to normal
        assert_eq!(deserialized.a, 4.into());
        // ... and the unwrapped field is unchanged
        assert_eq!(deserialized.b, 9);
    }
}
