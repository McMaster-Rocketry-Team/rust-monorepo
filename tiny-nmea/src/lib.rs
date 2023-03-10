#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod message;
mod gll;
mod common;
mod gsv;

use heapless::String;
use crate::message::{NMEAMessage, Time};
use heapless::Vec;
use crate::message::NMEAMessage::{GLL, GSV};

fn validate(sentence: &String<82>) -> Result<(), ()> {
    let expected_checksum = substring!(sentence, sentence.len() - 2, 2);
    let expected_checksum = u8::from_str_radix(expected_checksum.as_str(), 16).unwrap();
    let mut checksum: u8 = 0;
    sentence.bytes().skip(1).take(sentence.len() - 4).for_each(|byte| {
        checksum ^= byte;
    });
    if checksum != expected_checksum {
        return Err(());
    }

    Ok(())
}

pub fn parse(sentence: &String<82>) -> Result<NMEAMessage, ()> {
    validate(&sentence)?;
    let fields: Vec<&str, 41> = sentence.split(|c| c == '$' || c == ',' || c == '*').collect();
    let message_type = &(fields[1].clone())[2..5];
    match message_type {
        "GLL" => gll::parse_gll(fields),
        "GSV" => gsv::parse_gsv(fields),
        _ => Err(()),
    }
}

#[derive(Debug, Clone)]
pub struct NMEA {
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub utc: Option<Time>,
    pub satellites_visible: Option<u8>,
}

impl NMEA {
    pub fn new() -> Self {
        NMEA {
            latitude: None,
            longitude: None,
            utc: None,
            satellites_visible: None,
        }
    }

    pub fn update(&mut self, sentence: &String<82>) -> Result<(), ()> {
        match parse(sentence) {
            Ok(GLL { latitude, longitude, utc, .. }) => {
                self.latitude = Some(latitude);
                self.longitude = Some(longitude);
                self.utc = Some(utc);
                Ok(())
            }
            Ok(GSV { satellites_visible, .. }) => {
                self.satellites_visible = Some(satellites_visible);
                Ok(())
            }
            _ => Err(())
        }
    }

    pub fn has_fix(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }
}


#[cfg(test)]
mod tests {
    use alloc::format;
    use super::*;

    #[test]
    fn gll() {
        let result = parse(&String::from("$GNGLL,4315.68533,N,07955.20234,W,080023.000,A,A*5D")).unwrap();
        assert_eq!(format!("{:?}", result), "GLL { talker: \"GN\", latitude: 43.26142, longitude: -79.92004, utc: Time { hour: 8, minute: 0, second: 23, millisecond: 0 } }");
    }

    #[test]
    fn gsv() {
        let result = parse(&String::from("$GPGSV,2,2,07,23,62,115,24,24,42,057,20,32,52,272,21*4A")).unwrap();
        assert_eq!(format!("{:?}", result), "GSV { talker: \"GP\", satellites_visible: 7 }");
    }
}
