use packed_struct::prelude::*;

pub trait VariableIntTrait {
    type Base;
    type Packed;
}

pub struct VariableInt<const BITS: usize>;

macro_rules! impl_variable_int {
    ($base_type: ty, $bits:literal) => {
        impl VariableIntTrait for VariableInt<$bits> {
            type Base = $base_type;
            type Packed = Integer<$base_type, packed_bits::Bits<$bits>>;
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