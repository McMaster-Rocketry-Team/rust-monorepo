use core::{
    cell::RefCell,
    ops::Deref,
};

use crate::driver::gps::{NmeaSentence, GPS};
use chrono::{TimeZone, Utc};
use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex},
    signal::Signal,
};
use futures::join;
use nmea::Nmea;
use rkyv::{Archive, Deserialize, Serialize};

use super::delta_factory::Deltable;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
pub struct GPSLocation {
    pub timestamp: f64,
    pub gps_timestamp: Option<i64>, // in seconds
    pub lat_lon: Option<(f64, f64)>,
    pub altitude: Option<f32>,
    pub num_of_fix_satellites: u8,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
pub struct GPSLocationDelta {
    pub timestamp: u8,
    pub gps_timestamp: u8,
    pub lat_lon: (u16, u16),
    pub altitude: u8,
    pub num_of_fix_satellites: u8,
    pub hdop: u8,
    pub vdop: u8,
    pub pdop: u8,
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 100.0, f64, u8);
    fixed_point_factory!(LatLon, -0.001, 0.001, f64, u16);
    fixed_point_factory!(Altitude, -40.0, 40.0, f32, u8);
    fixed_point_factory!(DoP, 0.0, 40.0, f32, u8);
}

impl Deltable for GPSLocation {
    type DeltaType = GPSLocationDelta;

    fn add_delta(&self, delta: &GPSLocationDelta) -> Option<Self> {
        Some(Self {
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            gps_timestamp: Some(self.gps_timestamp? + delta.gps_timestamp as i64),
            lat_lon: Some((
                self.lat_lon?.0 + factories::LatLon::to_float(delta.lat_lon.0),
                self.lat_lon?.1 + factories::LatLon::to_float(delta.lat_lon.1),
            )),
            altitude: Some(self.altitude? + factories::Altitude::to_float(delta.altitude)),
            num_of_fix_satellites: self.num_of_fix_satellites + delta.num_of_fix_satellites,
            hdop: Some(self.hdop? + factories::DoP::to_float(delta.hdop)),
            vdop: Some(self.vdop? + factories::DoP::to_float(delta.vdop)),
            pdop: Some(self.pdop? + factories::DoP::to_float(delta.pdop)),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        Some(GPSLocationDelta {
            timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
            gps_timestamp: (self.gps_timestamp? - other.gps_timestamp?) as u8,
            lat_lon: (
                factories::LatLon::to_fixed_point(self.lat_lon?.0 - other.lat_lon?.0)?,
                factories::LatLon::to_fixed_point(self.lat_lon?.1 - other.lat_lon?.1)?,
            ),
            altitude: factories::Altitude::to_fixed_point(self.altitude? - other.altitude?)?,
            num_of_fix_satellites: self.num_of_fix_satellites - other.num_of_fix_satellites,
            hdop: factories::DoP::to_fixed_point(self.hdop? - other.hdop?)?,
            vdop: factories::DoP::to_fixed_point(self.vdop? - other.vdop?)?,
            pdop: factories::DoP::to_fixed_point(self.pdop? - other.pdop?)?,
        })
    }
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
            num_of_fix_satellites: value.num_of_fix_satellites.unwrap_or(0) as u8,
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
                            // log_warn!(
                            //     "Failed to parse NMEA sentence {} {:?}",
                            //     &nmea_sentence
                            //         .sentence
                            //         .as_str()
                            //         .trim_end_matches(|c| c == '\r' || c == '\n'),
                            //     NmeaErrorWrapper(error),
                            // );
                        }
                    }
                });
            }
        };
        join!(read_fut, parse_fut);
        log_unreachable!();
    }
}

#[derive(defmt::Format)]
struct NmeaErrorWrapper<'a>(#[defmt(Debug2Format)] pub nmea::Error<'a>);
