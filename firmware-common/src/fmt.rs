#![macro_use]
#![allow(unused_macros)]

macro_rules! log_trace {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::trace!($s $(, $x)*);

            ::defmt::trace!($s $(, $x)*);
        }
    };
}

macro_rules! log_debug {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::debug!($s $(, $x)*);

            ::defmt::debug!($s $(, $x)*);
        }
    };
}

macro_rules! log_info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::info!($s $(, $x)*);

            ::defmt::info!($s $(, $x)*);
        }
    };
}

macro_rules! log_warn {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::warn!($s $(, $x)*);

            ::defmt::warn!($s $(, $x)*);
        }
    };
}

macro_rules! log_error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::error!($s $(, $x)*);

            ::defmt::error!($s $(, $x)*);
        }
    };
}
