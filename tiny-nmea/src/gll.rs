use crate::message::NMEAMessage;
use heapless::String;
use heapless::Vec;
use crate::common::parse_time;
use crate::message::NMEAMessage::GLL;
use crate::substring;

pub fn parse_gll(fields: Vec<&str, 41>) -> Result<NMEAMessage, ()> {
    let talker = substring!(fields[1], 0, 2);
    let lat_degree = substring!(fields[2], 0, 2).parse::<f32>().unwrap();
    let lat_minute = substring!(fields[2], 2, 10).parse::<f32>().unwrap();
    let mut lat = lat_degree + (lat_minute / 60.0);
    let lat_direction = fields[3];
    if lat_direction == "S" {
        lat = -lat;
    }

    let lon_degree = substring!(fields[4], 0, 3).parse::<f32>().unwrap();
    let lon_minute = substring!(fields[4], 3, 10).parse::<f32>().unwrap();
    let mut lon = lon_degree + (lon_minute / 60.0);
    let lon_direction = fields[5];
    if lon_direction == "W" {
        lon = -lon;
    }

    Ok(GLL{
        talker,
        latitude: lat,
        longitude: lon,
        utc: parse_time(fields[5]).unwrap(),
    })
}
