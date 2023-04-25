pub(super) fn find_most_common_u16_out_of_4(buffer:&[u8]) -> Option<u16> {
    find_most_common(
        u16::from_be_bytes((&buffer[0..2]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[2..4]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[4..6]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[6..8]).try_into().unwrap())
    )
}

fn find_most_common(a: u16, b: u16, c: u16, d: u16) -> Option<u16> {
    if a == b {
        return Some(a);
    }
    if a == c {
        return Some(a);
    }
    if a == d {
        return Some(a);
    }
    if b == c {
        return Some(b);
    }
    if b == d {
        return Some(b);
    }
    if c == d {
        return Some(c);
    }

    None
}