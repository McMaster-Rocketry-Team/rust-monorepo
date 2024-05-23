use core::{cell::RefCell, ops::Deref};

use crate::driver::gps::{NmeaSentence, GPS};
use chrono::{TimeZone, Utc};
use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex},
    signal::Signal,
};
use futures::join;
use nmea::Nmea;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
pub struct GPSLocation {
    pub timestamp: f64,
    pub gps_timestamp: Option<i64>, // in seconds
    pub lat_lon: Option<(f64, f64)>,
    pub altitude: Option<f32>,
    pub num_of_fix_satellites: u32,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl<T: Deref<Target = Nmea>> From<(f64, T)> for GPSLocation {
    fn from((timestamp, value): (f64, T)) -> Self {
        let lat_lon: Option<(f64, f64)> = if let Some(lat) = value.latitude
            && let Some(lon) = value.longitude
        {
            Some((lat, lon))
        } else {
            None
        };

        let gps_timestamp = if let Some(date) = value.fix_date
            && let Some(time) = value.fix_time
        {
            let datetime = date.and_time(time);
            let datetime = Utc.from_utc_datetime(&datetime);
            Some(datetime.timestamp())
        } else {
            None
        };

        Self {
            timestamp,
            gps_timestamp,
            lat_lon,
            altitude: value.altitude,
            num_of_fix_satellites: value.num_of_fix_satellites.unwrap_or(0),
            hdop: value.hdop,
            vdop: value.vdop,
            pdop: value.pdop,
        }
    }
}

pub struct GPSParser {
    nmea: BlockingMutex<NoopRawMutex, RefCell<(f64, Nmea)>>,
    updated: BlockingMutex<NoopRawMutex, RefCell<bool>>,
}

impl GPSParser {
    pub fn new() -> Self {
        Self {
            nmea: BlockingMutex::new(RefCell::new((0.0, Nmea::default()))),
            updated: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub fn get_nmea(&self) -> GPSLocation {
        self.updated.lock(|updated| {
            *updated.borrow_mut() = false;
        });
        self.nmea.lock(|nmea| {
            let nmea = nmea.borrow();
            (nmea.0, &nmea.1).into()
        })
    }

    pub fn get_updated(&self) -> bool {
        self.updated.lock(|updated| *updated.borrow())
    }

    pub async fn run(&self, gps: &mut impl GPS) -> ! {
        let signal = Signal::<NoopRawMutex, NmeaSentence>::new();

        let read_fut = async {
            loop {
                let nmea_sentence = gps.next_nmea_sentence().await;
                signal.signal(nmea_sentence);
            }
        };

        let parse_fut = async {
            loop {
                let nmea_sentence = signal.wait().await;
                log_trace!(
                    "NMEA sentence {}",
                    &nmea_sentence
                        .sentence
                        .as_str()
                        .trim_end_matches(|c| c == '\r' || c == '\n'),
                );
                self.nmea.lock(|nmea| {
                    let mut nmea = nmea.borrow_mut();
                    match nmea.1.parse(&nmea_sentence.sentence.as_str()) {
                        Ok(_) => {
                            nmea.0 = nmea_sentence.timestamp;
                            self.updated.lock(|updated| {
                                *updated.borrow_mut() = true;
                            });
                        }
                        Err(nmea::Error::DisabledSentence) => {}
                        Err(error) => {
                            log_warn!(
                                "Failed to parse NMEA sentence {} {:?}",
                                &nmea_sentence
                                    .sentence
                                    .as_str()
                                    .trim_end_matches(|c| c == '\r' || c == '\n'),
                                NmeaErrorWrapper(error),
                            );
                        }
                    }
                });
            }
        };
        join!(read_fut, parse_fut);
        defmt::unreachable!();
    }
}

#[derive(defmt::Format)]
struct NmeaErrorWrapper<'a>(#[defmt(Debug2Format)] pub nmea::Error<'a>);
