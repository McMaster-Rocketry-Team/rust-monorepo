use core::{cell::RefCell, ops::Deref};

use defmt::{debug, warn};
use nmea::Nmea;
use rkyv::{Archive, Deserialize, Serialize};

use crate::driver::{gps::GPS, timer::Timer};
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex};

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct GPSLocation {
    pub timestamp: f64,
    pub lat_lon: Option<(f64, f64)>,
    pub altitude: Option<f32>,
    pub num_of_fix_satellites: u32,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl<T: Deref<Target = Nmea>> From<(f64, T)> for GPSLocation {
    fn from((timestamp, value): (f64, T)) -> Self {
        let lat_lon = if let Some(lat) = value.latitude && let Some(lon) = value.longitude{
            Some((lat, lon))
        }else{
            None
        };

        Self {
            timestamp,
            lat_lon,
            altitude: value.altitude,
            num_of_fix_satellites: value.num_of_fix_satellites.unwrap_or(0),
            hdop: value.hdop,
            vdop: value.vdop,
            pdop: value.pdop,
        }
    }
}

pub struct GPSParser<T: Timer> {
    nmea: BlockingMutex<CriticalSectionRawMutex, RefCell<Nmea>>,
    updated: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
    timer: T,
}

impl<T: Timer> GPSParser<T> {
    pub fn new(timer: T) -> Self {
        Self {
            nmea: BlockingMutex::new(RefCell::new(Nmea::default())),
            updated: BlockingMutex::new(RefCell::new(false)),
            timer,
        }
    }

    pub fn get_nmea(&self) -> GPSLocation {
        self.updated.lock(|updated| {
            *updated.borrow_mut() = false;
        });
        self.nmea
            .lock(|nmea| (self.timer.now_mills(), nmea.borrow()).into())
    }

    pub fn get_updated(&self) -> bool {
        self.updated.lock(|updated| *updated.borrow())
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
                    self.updated.lock(|updated| {
                        *updated.borrow_mut() = true;
                    });
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
