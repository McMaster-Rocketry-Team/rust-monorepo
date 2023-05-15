macro_rules! packet {
    ($($elem:expr),*) => {{
        let mut buf = Vec::<u8, MAX_PAYLOAD_LENGTH>::new();
        $(
            buf.push($elem);
        )*

        buf
    }}
}
