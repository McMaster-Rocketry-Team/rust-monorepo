#![allow(dead_code)]
use libm::round;
use libm::roundf;
use packed_struct::prelude::*;

// FIXME remove
#[macro_export]
macro_rules! fixed_point_factory {
    ($name:ident, $min:expr, $max:expr, $float:ident, $fixed:ident) => {
        pub struct $name;

        impl $name {
            pub fn to_fixed_point(value: $float) -> Option<$fixed> {
                if value < $min || value > $max {
                    return None;
                }
                let value = value - $min;
                let value = value / ($max - $min);
                let value = value * $fixed::max_value() as $float;
                Some(value as $fixed)
            }

            pub fn to_fixed_point_capped(value: $float) -> $fixed {
                let value = if value < $min {
                    $min
                } else if value > $max {
                    $max
                } else {
                    value
                };
                return Self::to_fixed_point(value).unwrap();
            }

            pub fn to_float(value: $fixed) -> $float {
                let value = value as $float;
                let value = value / $fixed::max_value() as $float;
                let value = value * ($max - $min);
                value + $min
            }
        }
    };
}

pub struct FixedPointFactory<Source, const TARGET_BITS: usize> {
    // not used in code, but needed to show the number of bits when you hover over the type
    pub bits: u8,
    pub min: Source,
    pub max: Source,
}

macro_rules! fixed_point_factory_impl {
    ($source:ty, $target_base: ty, $target_bits: expr, $round_fn: ident) => {
        impl FixedPointFactory<$source, $target_bits> {
            pub type PackedInt = Integer<$target_base, packed_bits::Bits<$target_bits>>;

            pub const fn new(min: $source, max: $source, bits: u8) -> Self {
                Self { min, max, bits }
            }

            fn target_max() -> $target_base {
                (1 as $target_base)
                    .checked_shl($target_bits)
                    .unwrap_or(0)
                    .wrapping_sub(1)
            }

            pub fn to_fixed_point(
                &self,
                value: $source,
            ) -> Option<Self::PackedInt> {
                if value < self.min || value > self.max {
                    return None;
                }
                let value = value - self.min;
                let value = value / (self.max - self.min);
                let value = value * Self::target_max() as $source;
                Some(($round_fn(value) as $target_base).into())
            }

            pub fn to_fixed_point_capped(
                &self,
                value: $source,
            ) -> Self::PackedInt {
                let value = if value < self.min {
                    self.min
                } else if value > self.max {
                    self.max
                } else {
                    value
                };
                return self.to_fixed_point(value).unwrap();
            }

            pub fn to_float(
                &self,
                value: Self::PackedInt,
            ) -> $source {
                let value: $target_base = value.into();
                let value = value as $source;
                let value = value / Self::target_max() as $source;
                let value = value * (self.max - self.min);
                value + self.min
            }
        }
    };
}

fixed_point_factory_impl!(f64, u8, 1, round);
fixed_point_factory_impl!(f64, u8, 2, round);
fixed_point_factory_impl!(f64, u8, 3, round);
fixed_point_factory_impl!(f64, u8, 4, round);
fixed_point_factory_impl!(f64, u8, 5, round);
fixed_point_factory_impl!(f64, u8, 6, round);
fixed_point_factory_impl!(f64, u8, 7, round);
fixed_point_factory_impl!(f64, u8, 8, round);
fixed_point_factory_impl!(f64, u16, 9, round);
fixed_point_factory_impl!(f64, u16, 10, round);
fixed_point_factory_impl!(f64, u16, 11, round);
fixed_point_factory_impl!(f64, u16, 12, round);
fixed_point_factory_impl!(f64, u16, 13, round);
fixed_point_factory_impl!(f64, u16, 14, round);
fixed_point_factory_impl!(f64, u16, 15, round);
fixed_point_factory_impl!(f64, u16, 16, round);
fixed_point_factory_impl!(f64, u32, 17, round);
fixed_point_factory_impl!(f64, u32, 18, round);
fixed_point_factory_impl!(f64, u32, 19, round);
fixed_point_factory_impl!(f64, u32, 20, round);
fixed_point_factory_impl!(f64, u32, 21, round);
fixed_point_factory_impl!(f64, u32, 22, round);
fixed_point_factory_impl!(f64, u32, 23, round);
fixed_point_factory_impl!(f64, u32, 24, round);
fixed_point_factory_impl!(f64, u32, 25, round);
fixed_point_factory_impl!(f64, u32, 26, round);
fixed_point_factory_impl!(f64, u32, 27, round);
fixed_point_factory_impl!(f64, u32, 28, round);
fixed_point_factory_impl!(f64, u32, 29, round);
fixed_point_factory_impl!(f64, u32, 30, round);
fixed_point_factory_impl!(f64, u32, 31, round);
fixed_point_factory_impl!(f64, u32, 32, round);

fixed_point_factory_impl!(f32, u8, 1, roundf);
fixed_point_factory_impl!(f32, u8, 2, roundf);
fixed_point_factory_impl!(f32, u8, 3, roundf);
fixed_point_factory_impl!(f32, u8, 4, roundf);
fixed_point_factory_impl!(f32, u8, 5, roundf);
fixed_point_factory_impl!(f32, u8, 6, roundf);
fixed_point_factory_impl!(f32, u8, 7, roundf);
fixed_point_factory_impl!(f32, u8, 8, roundf);
fixed_point_factory_impl!(f32, u16, 9, roundf);
fixed_point_factory_impl!(f32, u16, 10, roundf);
fixed_point_factory_impl!(f32, u16, 11, roundf);
fixed_point_factory_impl!(f32, u16, 12, roundf);
fixed_point_factory_impl!(f32, u16, 13, roundf);
fixed_point_factory_impl!(f32, u16, 14, roundf);
fixed_point_factory_impl!(f32, u16, 15, roundf);
fixed_point_factory_impl!(f32, u16, 16, roundf);
fixed_point_factory_impl!(f32, u32, 17, roundf);
fixed_point_factory_impl!(f32, u32, 18, roundf);
fixed_point_factory_impl!(f32, u32, 19, roundf);
fixed_point_factory_impl!(f32, u32, 20, roundf);
fixed_point_factory_impl!(f32, u32, 21, roundf);
fixed_point_factory_impl!(f32, u32, 22, roundf);
fixed_point_factory_impl!(f32, u32, 23, roundf);
fixed_point_factory_impl!(f32, u32, 24, roundf);
fixed_point_factory_impl!(f32, u32, 25, roundf);
fixed_point_factory_impl!(f32, u32, 26, roundf);
fixed_point_factory_impl!(f32, u32, 27, roundf);
fixed_point_factory_impl!(f32, u32, 28, roundf);
fixed_point_factory_impl!(f32, u32, 29, roundf);
fixed_point_factory_impl!(f32, u32, 30, roundf);
fixed_point_factory_impl!(f32, u32, 31, roundf);
fixed_point_factory_impl!(f32, u32, 32, roundf);

#[macro_export]
macro_rules! define_const_fixed_point_factory {
    ($name:ident, $packed_int_name:ident, $source:ty, $min:literal, $max:literal, $max_error:literal) => {
        #[allow(non_upper_case_globals)]
        const $name: crate::common::fixed_point::FixedPointFactory::<$source, {calculate_required_bits::calculate_required_bits!($min, $max, $max_error) as usize}> = crate::common::fixed_point::FixedPointFactory::<$source, {calculate_required_bits::calculate_required_bits!($min, $max, $max_error) as usize}>::new($min, $max, calculate_required_bits::calculate_required_bits!($min, $max, $max_error) as u8);
        type $packed_int_name = crate::common::fixed_point::FixedPointFactory::<$source, {calculate_required_bits::calculate_required_bits!($min, $max, $max_error) as usize}>::PackedInt;
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use calculate_required_bits::calculate_required_bits;

    #[test]
    fn test_fixed_point_factory() {
        let factory = FixedPointFactory::<f64, 16>::new(0.0, 1.0, 16);
        assert_eq!(factory.to_fixed_point(0.0), Some(0.into()));
        assert_eq!(factory.to_fixed_point(0.5), Some(32768.into()));
        assert_eq!(factory.to_fixed_point(1.0), Some(65535.into()));
        assert_eq!(factory.to_fixed_point(-1.0), None);
        assert_eq!(factory.to_fixed_point(2.0), None);

        assert_relative_eq!(factory.to_float(0.into()), 0.0, epsilon = 0.0001);
        assert_relative_eq!(factory.to_float(32768.into()), 0.5, epsilon = 0.0001);
        assert_relative_eq!(factory.to_float(65535.into()), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn test_fixed_point_factory_one_bit() {
        define_const_fixed_point_factory!(factory, Ty, f32, 0.0, 1.0, 0.5);
        assert_eq!(factory.to_fixed_point(0.0), Some(0.into()));
        assert_eq!(factory.to_fixed_point(0.25), Some(0.into()));
        assert_eq!(factory.to_fixed_point(0.5), Some(1.into()));
        assert_eq!(factory.to_fixed_point(0.75), Some(1.into()));
        assert_eq!(factory.to_fixed_point(1.0), Some(1.into()));
        assert_eq!(factory.to_fixed_point(-1.0), None);
        assert_eq!(factory.to_fixed_point(2.0), None);

        assert_relative_eq!(factory.to_float(0.into()), 0.0, epsilon = 0.0001);
        assert_relative_eq!(factory.to_float(1.into()), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn test_calculate_required_bits() {
        assert_eq!(calculate_required_bits!(0.0, 1.0, 0.5), 1);
        assert_eq!(calculate_required_bits!(0.0, 1.0, 0.3), 2);
        assert_eq!(calculate_required_bits!(0.0, 1.0, 0.25), 2);
        assert_eq!(calculate_required_bits!(0.0, 1.0, 0.2), 3);
    }
}
