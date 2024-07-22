#![allow(dead_code)]
use packed_struct::prelude::*;

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
    pub min: Source,
    pub max: Source,
}

macro_rules! fixed_point_factory_impl {
    ($source:ty, $target_base: ty, $target_bits: expr) => {
        impl FixedPointFactory<$source, $target_bits> {
            pub fn new(min: $source, max: $source) -> Self {
                Self { min, max }
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
            ) -> Option<Integer<$target_base, packed_bits::Bits<$target_bits>>> {
                if value < self.min || value > self.max {
                    return None;
                }
                let value = value - self.min;
                let value = value / (self.max - self.min);
                let value = value * Self::target_max() as $source;
                Some((value as $target_base).into())
            }

            pub fn to_fixed_point_capped(
                &self,
                value: $source,
            ) -> Integer<$target_base, packed_bits::Bits<$target_bits>> {
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
                value: Integer<$target_base, packed_bits::Bits<$target_bits>>,
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

fixed_point_factory_impl!(f64, u8, 2);
fixed_point_factory_impl!(f64, u8, 3);
fixed_point_factory_impl!(f64, u8, 4);
fixed_point_factory_impl!(f64, u8, 5);
fixed_point_factory_impl!(f64, u8, 6);
fixed_point_factory_impl!(f64, u8, 7);
fixed_point_factory_impl!(f64, u8, 8);
fixed_point_factory_impl!(f64, u16, 9);
fixed_point_factory_impl!(f64, u16, 10);
fixed_point_factory_impl!(f64, u16, 11);
fixed_point_factory_impl!(f64, u16, 12);
fixed_point_factory_impl!(f64, u16, 13);
fixed_point_factory_impl!(f64, u16, 14);
fixed_point_factory_impl!(f64, u16, 15);
fixed_point_factory_impl!(f64, u16, 16);
fixed_point_factory_impl!(f64, u32, 17);
fixed_point_factory_impl!(f64, u32, 18);
fixed_point_factory_impl!(f64, u32, 19);
fixed_point_factory_impl!(f64, u32, 20);
fixed_point_factory_impl!(f64, u32, 21);
fixed_point_factory_impl!(f64, u32, 22);
fixed_point_factory_impl!(f64, u32, 23);
fixed_point_factory_impl!(f64, u32, 24);
fixed_point_factory_impl!(f64, u32, 25);
fixed_point_factory_impl!(f64, u32, 26);
fixed_point_factory_impl!(f64, u32, 27);
fixed_point_factory_impl!(f64, u32, 28);
fixed_point_factory_impl!(f64, u32, 29);
fixed_point_factory_impl!(f64, u32, 30);
fixed_point_factory_impl!(f64, u32, 31);
fixed_point_factory_impl!(f64, u32, 32);

fixed_point_factory_impl!(f32, u8, 2);
fixed_point_factory_impl!(f32, u8, 3);
fixed_point_factory_impl!(f32, u8, 4);
fixed_point_factory_impl!(f32, u8, 5);
fixed_point_factory_impl!(f32, u8, 6);
fixed_point_factory_impl!(f32, u8, 7);
fixed_point_factory_impl!(f32, u8, 8);
fixed_point_factory_impl!(f32, u16, 9);
fixed_point_factory_impl!(f32, u16, 10);
fixed_point_factory_impl!(f32, u16, 11);
fixed_point_factory_impl!(f32, u16, 12);
fixed_point_factory_impl!(f32, u16, 13);
fixed_point_factory_impl!(f32, u16, 14);
fixed_point_factory_impl!(f32, u16, 15);
fixed_point_factory_impl!(f32, u16, 16);
fixed_point_factory_impl!(f32, u32, 17);
fixed_point_factory_impl!(f32, u32, 18);
fixed_point_factory_impl!(f32, u32, 19);
fixed_point_factory_impl!(f32, u32, 20);
fixed_point_factory_impl!(f32, u32, 21);
fixed_point_factory_impl!(f32, u32, 22);
fixed_point_factory_impl!(f32, u32, 23);
fixed_point_factory_impl!(f32, u32, 24);
fixed_point_factory_impl!(f32, u32, 25);
fixed_point_factory_impl!(f32, u32, 26);
fixed_point_factory_impl!(f32, u32, 27);
fixed_point_factory_impl!(f32, u32, 28);
fixed_point_factory_impl!(f32, u32, 29);
fixed_point_factory_impl!(f32, u32, 30);
fixed_point_factory_impl!(f32, u32, 31);
fixed_point_factory_impl!(f32, u32, 32);

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use crate::common::fixed_point::FixedPointFactory;

    #[test]
    fn test_fixed_point_factory() {
        let factory = FixedPointFactory::<f64, 16>::new(0.0, 1.0);
        assert_eq!(factory.to_fixed_point(0.0), Some(0.into()));
        assert_eq!(factory.to_fixed_point(0.5), Some(32767.into()));
        assert_eq!(factory.to_fixed_point(1.0), Some(65535.into()));
        assert_eq!(factory.to_fixed_point(-1.0), None);
        assert_eq!(factory.to_fixed_point(2.0), None);

        assert_relative_eq!(factory.to_float(0.into()), 0.0, epsilon = 0.0001);
        assert_relative_eq!(
            factory.to_float(32767.into()),
            0.5,
            epsilon = 0.0001
        );
        assert_relative_eq!(
            factory.to_float(65535.into()),
            1.0,
            epsilon = 0.0001
        );
    }
}
