use crate::common::delta_logger::prelude::*;
use crate::common::fixed_point::F32FixedPointFactory;
use crate::common::sensor_reading::{SensorData, SensorReading};
use crate::common::variable_int::{VariableInt, VariableIntTrait};
use crate::{
    common::{delta_logger::delta_factory::Deltable, ticker::Ticker},
    Clock, Delay,
};
use crate::{fixed_point_factory, fixed_point_factory_slope};
use chrono::{TimeZone as _, Utc};
use core::fmt::Debug;
use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use libm::floor;
use nmea::Nmea;

use super::timestamp::BootTimestamp;

#[derive(defmt::Format, Debug, Clone)]
pub struct GPSData {
    pub timestamp: Option<i64>, // in seconds
    pub lat_lon: Option<(f64, f64)>,
    pub altitude: Option<f32>,
    pub num_of_fix_satellites: u8,
    pub hdop: Option<f32>,
    pub vdop: Option<f32>,
    pub pdop: Option<f32>,
}

impl From<&Nmea> for GPSData {
    fn from(nmea: &Nmea) -> Self {
        let lat_lon: Option<(f64, f64)> =
            if let (Some(lat), Some(lon)) = (nmea.latitude, nmea.longitude) {
                Some((lat, lon))
            } else {
                None
            };

        let timestamp = if let (Some(date), Some(time)) = (nmea.fix_date, nmea.fix_time) {
            let datetime = date.and_time(time);
            let datetime = Utc.from_utc_datetime(&datetime);
            Some(datetime.timestamp())
        } else {
            None
        };

        Self {
            timestamp,
            lat_lon,
            altitude: nmea.altitude,
            num_of_fix_satellites: nmea.num_of_fix_satellites.unwrap_or(0) as u8,
            hdop: nmea.hdop,
            vdop: nmea.vdop,
            pdop: nmea.pdop,
        }
    }
}

impl BitArraySerializable for GPSData {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
        writer.write(self.lat_lon);
        writer.write(self.altitude);
        writer.write(self.num_of_fix_satellites);
        writer.write(self.hdop);
        writer.write(self.vdop);
        writer.write(self.pdop);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        GPSData {
            timestamp: reader.read().unwrap(),
            lat_lon: reader.read().unwrap(),
            altitude: reader.read().unwrap(),
            num_of_fix_satellites: reader.read().unwrap(),
            hdop: reader.read().unwrap(),
            vdop: reader.read().unwrap(),
            pdop: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        <Option<i64> as BitSlicePrimitive>::len_bits()
            + <Option<(f64, f64)> as BitSlicePrimitive>::len_bits()
            + <Option<f32> as BitSlicePrimitive>::len_bits()
            + u8::len_bits()
            + <Option<f32> as BitSlicePrimitive>::len_bits()
            + <Option<f32> as BitSlicePrimitive>::len_bits()
            + <Option<f32> as BitSlicePrimitive>::len_bits()
    }
}

fixed_point_factory_slope!(LatLonFac, 0.01, 100.0, 0.000005);
fixed_point_factory_slope!(AltitudeFac, 400.0, 100.0, 0.5);
fixed_point_factory!(DoPFac, f32, 0.0, 1.0, 0.1);

#[derive(defmt::Format, Debug, Clone)]
pub struct GPSDataDelta {
    pub timestamp: u8,
    #[defmt(Debug2Format)]
    pub lat_lon: (LatLonFacPacked, LatLonFacPacked),
    #[defmt(Debug2Format)]
    pub altitude: AltitudeFacPacked,
    #[defmt(Debug2Format)]
    pub num_of_fix_satellites: <VariableInt<2> as VariableIntTrait>::Packed,
    #[defmt(Debug2Format)]
    pub hdop: DoPFacPacked,
    #[defmt(Debug2Format)]
    pub vdop: DoPFacPacked,
    #[defmt(Debug2Format)]
    pub pdop: DoPFacPacked,
}

impl BitArraySerializable for GPSDataDelta {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
        writer.write(self.lat_lon);
        writer.write(self.altitude);
        writer.write(self.num_of_fix_satellites);
        writer.write(self.hdop);
        writer.write(self.vdop);
        writer.write(self.pdop);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
            lat_lon: reader.read().unwrap(),
            altitude: reader.read().unwrap(),
            num_of_fix_satellites: reader.read().unwrap(),
            hdop: reader.read().unwrap(),
            vdop: reader.read().unwrap(),
            pdop: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        u8::len_bits()
            + <(LatLonFacPacked, LatLonFacPacked)>::len_bits()
            + AltitudeFacPacked::len_bits()
            + <VariableInt<2> as VariableIntTrait>::Packed::len_bits()
            + DoPFacPacked::len_bits()
            + DoPFacPacked::len_bits()
            + DoPFacPacked::len_bits()
    }
}

impl Deltable for GPSData {
    type DeltaType = GPSDataDelta;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        let num_of_fix_satellites = match Into::<u8>::into(delta.num_of_fix_satellites) {
            0 => self.num_of_fix_satellites,
            1 => self.num_of_fix_satellites + 1,
            2 => {
                if self.num_of_fix_satellites >= 1 {
                    self.num_of_fix_satellites - 1
                } else {
                    0
                }
            }
            3 => {
                if self.num_of_fix_satellites >= 2 {
                    self.num_of_fix_satellites - 2
                } else {
                    0
                }
            }
            _ => {
                log_unreachable!();
            }
        };

        Some(Self {
            timestamp: Some(self.timestamp? + delta.timestamp as i64),
            lat_lon: Some((
                self.lat_lon?.0 + LatLonFac::to_float(delta.lat_lon.0) as f64,
                self.lat_lon?.1 + LatLonFac::to_float(delta.lat_lon.1) as f64,
            )),
            altitude: Some(self.altitude? + AltitudeFac::to_float(delta.altitude)),
            num_of_fix_satellites,
            hdop: Some(self.hdop? + DoPFac::to_float(delta.hdop)),
            vdop: Some(self.vdop? + DoPFac::to_float(delta.vdop)),
            pdop: Some(self.pdop? + DoPFac::to_float(delta.pdop)),
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        let timestamp = self.timestamp? - other.timestamp?;
        if timestamp > u8::MAX as i64 {
            return None;
        }

        let num_of_fix_satellites: <VariableInt<2> as VariableIntTrait>::Packed =
            match self.num_of_fix_satellites as i8 - other.num_of_fix_satellites as i8 {
                0 => 0.into(),
                1 => 1.into(),
                -1 => 2.into(),
                -2 => 3.into(),
                _ => {
                    return None;
                }
            };

        Some(Self::DeltaType {
            timestamp: timestamp as u8,
            lat_lon: (
                LatLonFac::to_fixed_point((self.lat_lon?.0 - other.lat_lon?.0) as f32)?,
                LatLonFac::to_fixed_point((self.lat_lon?.1 - other.lat_lon?.1) as f32)?,
            ),
            altitude: AltitudeFac::to_fixed_point(self.altitude? - other.altitude?)?,
            num_of_fix_satellites,
            hdop: DoPFac::to_fixed_point(self.hdop? - other.hdop?)?,
            vdop: DoPFac::to_fixed_point(self.vdop? - other.vdop?)?,
            pdop: DoPFac::to_fixed_point(self.pdop? - other.pdop?)?,
        })
    }
}

impl SensorData for GPSData {}

pub trait GPS {
    type Error: defmt::Format + Debug;

    async fn next_location(
        &mut self,
    ) ->  Result<SensorReading<BootTimestamp, GPSData>, Self::Error>;
}

pub trait GPSPPS {
    async fn wait_for_pps(&mut self);
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
            ticker: Ticker::every_starts_at(clock, delay, 1000.0, floor(now / 1000.0) * 1000.0),
        }
    }
}

impl<D: Delay, C: Clock> GPSPPS for DummyGPSPPS<D, C> {
    async fn wait_for_pps(&mut self) {
        self.ticker.next_skip_missed().await;
    }
}
