use crate::message::Time;
use heapless::String;

#[macro_export]
macro_rules! substring {
    ($string:expr, $start:expr, $length:expr) => {
        $string
            .chars()
            .skip($start)
            .take($length)
            .collect::<String<$length>>()
    };
    ($string:expr, $start:expr, $length:expr, $max_length:expr) => {
        $string
            .chars()
            .skip($start)
            .take($length)
            .collect::<String<$max_length>>()
    };
}

pub fn parse_time(str: &str) -> Result<Time, ()> {
    let hour = substring!(str, 0, 2).parse::<u8>().map_err(|_| ())?;
    let minute = substring!(str, 2, 2).parse::<u8>().map_err(|_| ())?;
    let second = substring!(str, 4, 2).parse::<u8>().map_err(|_| ())?;
    let millisecond = substring!(str, 7, 2).parse::<u8>().map_err(|_| ())?;
    Ok(Time {
        hour,
        minute,
        second,
        millisecond,
    })
}
