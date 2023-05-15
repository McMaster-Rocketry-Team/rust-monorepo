macro_rules! packet {
    ($($elem:expr),*) => {{
        let mut buf = Vec::<u8, 256>::new();
        $(
            buf.push($elem);
        )*

        buf
    }}
}
