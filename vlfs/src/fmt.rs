#![macro_use]
#![allow(unused_macros)]

macro_rules! log_trace {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::trace!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::trace!($s $(, $x)*);
        }
    };
}

macro_rules! log_debug {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::debug!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::debug!($s $(, $x)*);
        }
    };
}

macro_rules! log_info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::info!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::info!($s $(, $x)*);
        }
    };
}

macro_rules! log_warn {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::warn!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::warn!($s $(, $x)*);
        }
    };
}

macro_rules! log_error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::error!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::error!($s $(, $x)*);
        }
    };
}

macro_rules! log_panic {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::core::panic!($s $(, $x)*);

            #[cfg(not(feature = "log"))]
            ::defmt::panic!($s $(, $x)*);
        }
    };
}
