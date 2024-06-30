use crate::{
    common::{delta_factory::Deltable, ticker::Ticker},
    Clock, Delay,
};
use chrono::{TimeZone as _, Utc};
use core::fmt::Debug;
use core::future::Future;
use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use nmea::Nmea;
use rkyv::{Archive, Deserialize, Serialize};

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

impl GPSLocation {
    pub fn from_nmea(nmea: &Nmea, timestamp: f64) -> GPSLocation {
        let lat_lon: Option<(f64, f64)> = if let Some(lat) = nmea.latitude
            && let Some(lon) = nmea.longitude
        {
            Some((lat, lon))
        } else {
            None
        };

        let gps_timestamp = if let Some(date) = nmea.fix_date
            && let Some(time) = nmea.fix_time
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
            altitude: nmea.altitude,
            num_of_fix_satellites: nmea.num_of_fix_satellites.unwrap_or(0) as u8,
            hdop: nmea.hdop,
            vdop: nmea.vdop,
            pdop: nmea.pdop,
        }
    }
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

pub trait GPS {
    type Error: defmt::Format + Debug;

    fn next_location(&mut self) -> impl Future<Output = Result<GPSLocation, Self::Error>>;
}

pub trait GPSPPS {
    fn wait_for_pps(&mut self) -> impl Future<Output = ()>;
}

impl<'a, M, T> GPSPPS for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: GPSPPS,
{
    async fn wait_for_pps(&mut self) {
        self.deref_mut().wait_for_pps().await;
    }
}

pub struct DummyGPSPPS<D: Delay, C: Clock> {
    ticker: Ticker<C, D>,
}

impl<D: Delay, C: Clock> DummyGPSPPS<D, C> {
    pub fn new(delay: D, clock: C) -> Self {
        let now = clock.now_ms();
        Self {
            ticker: Ticker::every_starts_at(clock, delay, 1000.0, (now / 1000.0).floor() * 1000.0),
        }
    }
}

impl<D: Delay, C: Clock> GPSPPS for DummyGPSPPS<D, C> {
    async fn wait_for_pps(&mut self) {
        self.ticker.next_skip_missed().await;
    }
}
