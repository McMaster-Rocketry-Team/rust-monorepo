#![cfg_attr(not(test), no_std)]

mod common;
mod gll;
mod gsv;
mod message;

use crate::message::NMEAMessage::{GLL, GSV};
use crate::message::{NMEAMessage, Time};
use heapless::String;
use heapless::Vec;

fn validate(sentence: &String<84>) -> Result<(), ()> {
    if sentence.len() < 6 {
        return Err(());
    }
    let expected_checksum = substring!(sentence, sentence.len() - 4, 2);
    let expected_checksum = u8::from_str_radix(expected_checksum.as_str(), 16).map_err(|_| ())?;
    let mut checksum: u8 = 0;
    sentence
        .bytes()
        .skip(1)
        .take(sentence.len() - 6)
        .for_each(|byte| {
            checksum ^= byte;
        });
    if checksum != expected_checksum {
        return Err(());
    }

    Ok(())
}

pub fn parse(sentence: &String<84>) -> Result<NMEAMessage, ()> {
    validate(&sentence)?;
    let fields: Vec<&str, 41> = sentence
        .split(|c| c == '$' || c == ',' || c == '*')
        .collect();
    let message_type = fields[1].clone();
    if message_type.len() < 5 {
        return Err(());
    }
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

impl core::fmt::Display for NMEA {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "NMEA {{ latitude: {:?}, longitude: {:?}, utc: {:?}, satellites_visible: {:?} }}",
            self.latitude, self.longitude, self.utc, self.satellites_visible
        )
    }
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

    pub fn update(&mut self, sentence: &String<84>) -> Result<(), ()> {
        match parse(sentence) {
            Ok(GLL {
                latitude,
                longitude,
                utc,
                ..
            }) => {
                self.latitude = Some(latitude);
                self.longitude = Some(longitude);
                self.utc = Some(utc);
                Ok(())
            }
            Ok(GSV {
                satellites_visible, ..
            }) => {
                self.satellites_visible = Some(satellites_visible);
                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn has_fix(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::format;

    #[test]
    fn gll() {
        let result = parse(&String::try_from(
            "$GNGLL,4315.68533,N,07955.20234,W,080023.000,A,A*5D\r\n",
        ).unwrap())
        .unwrap();
        assert_eq!(format!("{:?}", result), "GLL { talker: \"GN\", latitude: 43.26142, longitude: -79.92004, utc: Time { hour: 8, minute: 0, second: 23, millisecond: 0 } }");
    }

    #[test]
    fn gsv() {
        let result = parse(&String::try_from(
            "$GPGSV,2,2,07,23,62,115,24,24,42,057,20,32,52,272,21*4A\r\n",
        ).unwrap())
        .unwrap();
        assert_eq!(
            format!("{:?}", result),
            "GSV { talker: \"GP\", satellites_visible: 7 }"
        );
    }
}
