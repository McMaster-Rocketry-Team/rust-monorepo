#[macro_export]
macro_rules! try_or_warn {
    ($e: expr) => {{
        if let Err(e) = $e {
            defmt::warn!("`{}` failed: {:?}", stringify!($e), e);
        }
    }}
}