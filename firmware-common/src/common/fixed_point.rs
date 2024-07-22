#![allow(dead_code)]

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

#[macro_export]
macro_rules! fixed_point_factory2 {
    ($name:ident, f32, $min:literal, $max:literal, $max_error:literal) => {
        fixed_point_factory2!($name, minmax, f32, libm::roundf, $min, $max, $max_error);
    };
    ($name:ident, f64, $min:literal, $max:literal, $max_error:literal) => {
        fixed_point_factory2!($name, minmax, f64, libm::round, $min, $max, $max_error);
    };
    ($name:ident, $mode: ident, $source: ty, $round_fn: path, $min:literal, $max:literal, $max_error:literal) => {
        calculate_required_bits::calculate_required_bits_docstr!($mode, $min, $max, $max_error, $name);

        impl $name {
            pub type Packed = <crate::common::variable_int::VariableInt<
                {
                    calculate_required_bits::calculate_required_bits!($mode, $min, $max, $max_error)
                        as usize
                },
            > as crate::common::variable_int::VariableIntTrait>::Packed;

            type _Base = <crate::common::variable_int::VariableInt<
                {
                    calculate_required_bits::calculate_required_bits!($mode, $min, $max, $max_error)
                        as usize
                },
            > as crate::common::variable_int::VariableIntTrait>::Base;

            fn _target_max() -> Self::_Base {
                num_traits::cast::<u8, Self::_Base>(1)
                    .unwrap()
                    .checked_shl(calculate_required_bits::calculate_required_bits!(
                        $mode, $min, $max, $max_error
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

            pub fn to_fixed_point_capped(value: $source) -> Self::Packed {
                let value = if value < $min {
                    $min
                } else if value > $max {
                    $max
                } else {
                    value
                };
                return Self::to_fixed_point(value).unwrap();
            }

            pub fn to_float(value: Self::Packed) -> $source {
                let value: Self::_Base = value.into();
                let value = value as $source;
                let value = value / Self::_target_max() as $source;
                let value = value * ($max - $min);
                value + $min
            }
        }

        paste::paste! {
            type [<$name Packed>] = $name::Packed;
        }
    };
}

#[macro_export]
/// threshold_slope is in unit / second
macro_rules! fixed_point_factory_slope {
    ($name:ident, $threshold_slope:literal, $sample_time_ms:literal, $max_error:literal) => {
        fixed_point_factory2!(
            $name,
            slope,
            f32,
            libm::roundf,
            $threshold_slope,
            $sample_time_ms,
            $max_error
        );
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use calculate_required_bits::calculate_required_bits;

    #[test]
    fn test_fixed_point_factory_one_bit() {
        fixed_point_factory2!(Factory, f32, 0.0, 1.0, 0.5);
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
}
