use core::cell::RefCell;

use defmt::{debug, warn};
use nmea::Nmea;

use crate::driver::gps::GPS;
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex};

pub struct GPSParser {
    nmea: BlockingMutex<CriticalSectionRawMutex, RefCell<Nmea>>,
}

impl GPSParser {
    pub fn new() -> Self {
        Self {
            nmea: BlockingMutex::new(RefCell::new(Nmea::default())),
        }
    }

    pub fn get_nmea(&self) -> Nmea {
        self.nmea.lock(|nmea| nmea.borrow().clone())
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
