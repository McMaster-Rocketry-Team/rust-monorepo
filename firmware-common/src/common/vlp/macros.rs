macro_rules! packet {
    ($($elem:expr),*) => {{
        let mut buf = Vec::<u8, 222>::new();
        $(
            buf.push($elem);
        )*

        buf
    }}
}
