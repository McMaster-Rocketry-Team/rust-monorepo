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

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    fixed_point_factory!(FixedPointFactoryf64u16, 0.0, 1.0, f64, u16);

    #[test]
    fn test_fixed_point_factory() {
        assert_eq!(
            FixedPointFactoryf64u16::to_fixed_point(0.0),
            Some(0)
        );
        assert_eq!(
            FixedPointFactoryf64u16::to_fixed_point(0.5),
            Some(32767)
        );
        assert_eq!(
            FixedPointFactoryf64u16::to_fixed_point(1.0),
            Some(65535)
        );
        assert_eq!(FixedPointFactoryf64u16::to_fixed_point(-1.0), None);
        assert_eq!(FixedPointFactoryf64u16::to_fixed_point(2.0), None);

        assert_relative_eq!(
            FixedPointFactoryf64u16::to_float(0),
            0.0,
            epsilon = 0.0001
        );
        assert_relative_eq!(
            FixedPointFactoryf64u16::to_float(32767),
            0.5,
            epsilon = 0.0001
        );
        assert_relative_eq!(
            FixedPointFactoryf64u16::to_float(65535),
            1.0,
            epsilon = 0.0001
        );
    }
}
