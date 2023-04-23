#[macro_export]
macro_rules! heapless_format {
    ($max_str_length: expr, $dst: expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        let mut line = heapless::String::<$max_str_length>::new();
        core::write!(&mut line, $dst, $($arg)*).unwrap();
        line
    }};
}

#[macro_export]
macro_rules! heapless_format_bytes {
    ($max_str_length: expr, $dst: expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        let mut line = heapless::String::<$max_str_length>::new();
        core::write!(&mut line, $dst, $($arg)*).unwrap();
        line
    }.as_bytes()};
}
