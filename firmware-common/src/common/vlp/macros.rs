#[macro_export]
macro_rules! packet {
    ($($elem:expr),*) => {{

        // max payload length is 222 im fucking done with this
        let mut buf = ::heapless::Vec::<u8, 222>::new();
        $(
            buf.push($elem);
        )*

        buf
    }}
}
