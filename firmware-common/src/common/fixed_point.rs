#![allow(dead_code)]

use super::variable_int::VariableIntTrait;

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

pub trait F32FixedPointFactory: Clone {
    type VI: VariableIntTrait;
    fn to_fixed_point(value: f32) -> Option<<Self::VI as VariableIntTrait>::Packed>;
    fn to_fixed_point_capped(value: f32) -> <Self::VI as VariableIntTrait>::Packed;
    fn to_float(value: <Self::VI as VariableIntTrait>::Packed) -> f32;
}

pub trait F64FixedPointFactory: Clone {
    type VI: VariableIntTrait;
    fn to_fixed_point(value: f64) -> Option<<Self::VI as VariableIntTrait>::Packed>;
    fn to_fixed_point_capped(value: f64) -> <Self::VI as VariableIntTrait>::Packed;
    fn to_float(value: <Self::VI as VariableIntTrait>::Packed) -> f64;
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
        calculate_required_bits::calculate_required_bits_docstr!(
            $mode, $min, $max, $max_error, $name
        );

        paste::paste! {
            type [<$name Packed>] = <crate::common::variable_int::VariableInt<
            {
                calculate_required_bits::calculate_required_bits!($mode, $min, $max, $max_error)
                    as usize
            },
            > as crate::common::variable_int::VariableIntTrait>::Packed;

            type [<$name Base>] = <crate::common::variable_int::VariableInt<
                {
                    calculate_required_bits::calculate_required_bits!($mode, $min, $max, $max_error)
                        as usize
                },
            > as crate::common::variable_int::VariableIntTrait>::Base;

            impl crate::common::fixed_point::[< $source:upper FixedPointFactory >] for $name {
                type VI = crate::common::variable_int::VariableInt<
                    {
                        calculate_required_bits::calculate_required_bits!($mode, $min, $max, $max_error)
                            as usize
                    },
                >;

                fn to_fixed_point(value: $source) -> Option<[<$name Packed>]> {
                    if value < calculate_required_bits::calculate_min!($mode, $min, $max, $max_error) || value > calculate_required_bits::calculate_max!($mode, $min, $max, $max_error) {
                        return None;
                    }
                    let value = value - calculate_required_bits::calculate_min!($mode, $min, $max, $max_error);
                    let value = value / (calculate_required_bits::calculate_max!($mode, $min, $max, $max_error) - calculate_required_bits::calculate_min!($mode, $min, $max, $max_error));
                    let value = value * Self::_target_max() as $source;
                    Some(
                        num_traits::cast::<$source, [<$name Base>]>($round_fn(value))
                            .unwrap()
                            .into(),
                    )
                }
                fn to_fixed_point_capped(value: $source) -> [<$name Packed>] {
                    let value = if value < calculate_required_bits::calculate_min!($mode, $min, $max, $max_error) {
                        calculate_required_bits::calculate_min!($mode, $min, $max, $max_error)
                    } else if value > calculate_required_bits::calculate_max!($mode, $min, $max, $max_error) {
                        calculate_required_bits::calculate_max!($mode, $min, $max, $max_error)
                    } else {
                        value
                    };
                    return Self::to_fixed_point(value).unwrap();
                }
                fn to_float(value: [<$name Packed>]) -> $source {
                    let value: [<$name Base>] = value.into();
                    let value = value as $source;
                    let value = value / Self::_target_max() as $source;
                    let value = value * (calculate_required_bits::calculate_max!($mode, $min, $max, $max_error) - calculate_required_bits::calculate_min!($mode, $min, $max, $max_error));
                    value + calculate_required_bits::calculate_min!($mode, $min, $max, $max_error)
                }
            }

            impl $name {
                fn _target_max() -> [<$name Base>] {
                    num_traits::cast::<u8, [<$name Base>]>(1)
                        .unwrap()
                        .checked_shl(calculate_required_bits::calculate_required_bits!(
                            $mode, $min, $max, $max_error
                        ) as u32)
                        .unwrap_or(0)
                        .wrapping_sub(1)
                }
            }
        }
    };
}

#[macro_export]
/// threshold_slope is in unit / second
macro_rules! fixed_point_factory_slope {
    ($name:ident, $threshold_slope:literal, $sample_time_ms:literal, $max_error:literal) => {
        crate::fixed_point_factory2!(
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

    #[test]
    fn test_fixed_point_factory_one_bit() {
        fixed_point_factory2!(Factory, f32, 0.0, 1.0, 0.5);
        assert_eq!(Factory::to_fixed_point(0.0), Some(0.into()));
        assert_eq!(Factory::to_fixed_point(0.25), Some(0.into()));
        assert_eq!(Factory::to_fixed_point(0.5), Some(1.into()));
        assert_eq!(Factory::to_fixed_point(0.75), Some(1.into()));
        assert_eq!(Factory::to_fixed_point(1.0), Some(1.into()));
        assert_eq!(Factory::to_fixed_point(-1.0), None);
        assert_eq!(Factory::to_fixed_point(2.0), None);

        assert_relative_eq!(Factory::to_float(0.into()), 0.0, epsilon = 0.0001);
        assert_relative_eq!(Factory::to_float(1.into()), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn test_fixed_point_factory_slope() {
        fixed_point_factory_slope!(Factory, 1.0, 1000.0, 1.0);
        assert_eq!(Factory::to_fixed_point(-1.0), Some(0.into()));
        assert_eq!(Factory::to_fixed_point(0.0), Some(1.into()));
        assert_eq!(Factory::to_fixed_point(1.0), Some(1.into()));

        assert_relative_eq!(Factory::to_float(0.into()), -1.0, epsilon = 0.0001);
        assert_relative_eq!(Factory::to_float(1.into()), 1.0, epsilon = 0.0001);
    }
}
