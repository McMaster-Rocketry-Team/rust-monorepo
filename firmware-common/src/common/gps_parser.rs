use core::{cell::RefCell, ops::Deref};

use chrono::{NaiveDate, NaiveTime};
use defmt::{debug, warn};
use nmea::Nmea;

use crate::driver::gps::GPS;
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex};

#[derive(Debug, Clone)]
pub struct GPSLocation {
    pub fix_time: Option<NaiveTime>,
    pub fix_date: Option<NaiveDate>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
    pub speed_over_ground: Option<f32>,
    pub true_course: Option<f32>,
    pub num_of_fix_satellites: Option<u32>,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl<T: Deref<Target = Nmea>> From<T> for GPSLocation {
    fn from(value: T) -> Self {
        Self {
            fix_time: value.fix_time,
            fix_date: value.fix_date,
            latitude: value.latitude,
            longitude: value.longitude,
            altitude: value.altitude,
            speed_over_ground: value.speed_over_ground,
            true_course: value.true_course,
            num_of_fix_satellites: value.num_of_fix_satellites,
            hdop: value.hdop,
            vdop: value.vdop,
            pdop: value.pdop,
        }
    }
}

pub struct GPSParser {
    nmea: BlockingMutex<CriticalSectionRawMutex, RefCell<Nmea>>,
}

impl GPSParser {
    pub fn new() -> Self {
        Self {
            nmea: BlockingMutex::new(RefCell::new(Nmea::default())),
        }
    }

    pub fn get_nmea(&self) -> GPSLocation {
        self.nmea.lock(|nmea| nmea.borrow().into())
    }

    pub async fn run(&self, gps: &mut impl GPS) -> ! {
        loop {
            let nmea_sentence = gps.next_nmea_sentence().await;
            self.nmea.lock(|nmea| {
                let success = nmea
                    .borrow_mut()
                    .parse(&nmea_sentence.sentence.as_str())
                    .is_ok();
                if !success {
                    warn!(
                        "Failed to parse NMEA sentence {}",
                        &nmea_sentence
                            .sentence
                            .as_str()
                            .trim_end_matches(|c| c == '\r' || c == '\n')
                    );
                } else {
                    debug!(
                        "GPS: {}",
                        &nmea_sentence
                            .sentence
                            .as_str()
                            .trim_end_matches(|c| c == '\r' || c == '\n')
                    );
                }
            });
        }
    }
}
