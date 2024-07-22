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

struct Test;

impl Test {
    type A = usize;
}

#[macro_export]
macro_rules! fixed_point_factory2 {
    ($name:ident, f32, $min:literal, $max:literal, $max_error:literal) => {
        fixed_point_factory2!($name, f32, libm::roundf, $min, $max, $max_error);
    };
    ($name:ident, f64, $min:literal, $max:literal, $max_error:literal) => {
        fixed_point_factory2!($name, f64, libm::round, $min, $max, $max_error);
    };
    ($name:ident, $source: ty, $round_fn: path, $min:literal, $max:literal, $max_error:literal) => {
        calculate_required_bits::calculate_required_bits_docstr!($min, $max, $max_error, $name);
        
        impl $name {
            pub type Packed = <VariableInt<
                {
                    calculate_required_bits::calculate_required_bits!($min, $max, $max_error)
                        as usize
                },
            > as VariableIntTrait>::Packed;

            type _Base = <VariableInt<
                {
                    calculate_required_bits::calculate_required_bits!($min, $max, $max_error)
                        as usize
                },
            > as VariableIntTrait>::Base;

            fn _target_max() -> Self::_Base {
                num_traits::cast::<u8, Self::_Base>(1)
                    .unwrap()
                    .checked_shl(calculate_required_bits::calculate_required_bits!(
                        $min, $max, $max_error
                    ) as u32)
                    .unwrap_or(0)
                    .wrapping_sub(1)
            }

            pub fn to_fixed_point(value: $source) -> Option<Self::Packed> {
                if value < $min || value > $max {
                    return None;
                }
                let value = value - $min;
                let value = value / ($max - $min);
                let value = value * Self::_target_max() as $source;
                Some(
                    num_traits::cast::<$source, Self::_Base>($round_fn(value))
                        .unwrap()
                        .into(),
                )
            }

            pub fn to_fixed_point_capped(&self, value: $source) -> Self::Packed {
                let value = if value < $min {
                    $min
                } else if value > $max {
                    $max
                } else {
                    value
                };
                return Self::to_fixed_point(value).unwrap();
            }

            pub fn to_float(&self, value: Self::Packed) -> $source {
                let value: Self::_Base = value.into();
                let value = value as $source;
                let value = value / Self::_target_max() as $source;
                let value = value * ($max - $min);
                value + $min
            }
        }
    };
}

fixed_point_factory2!(Test2, f32, -0.2, 2.2, 0.01);
