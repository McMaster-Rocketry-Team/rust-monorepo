use core::cell::{RefCell, RefMut};

use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    common::unix_clock::UnixClock,
    driver::gps::GPSLocation,
    Clock,
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};

use super::packet::VLPDownlinkPacket;

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(BatteryV, 2.0 * 2.0, 4.3 * 2.0, f32, u16);
    fixed_point_factory!(Temperature, -10.0, 85.0, f32, u16);
    fixed_point_factory!(Altitude, -50.0, 5000.0, f32, u16);
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct PadIdleTelemetryPacket {
    timestamp: f64,
    lat_lon: (f64, f64),
    battery_v: u16,
    temperature: u16,
    software_armed: bool,
}

impl PadIdleTelemetryPacket {
    pub fn new(
        timestamp: f64,
        lat_lon: (f64, f64),
        battery_v: f32,
        temperature: f32,
        software_armed: bool,
    ) -> Self {
        Self {
            timestamp,
            lat_lon,
            battery_v: factories::BatteryV::to_fixed_point_capped(battery_v),
            temperature: factories::Temperature::to_fixed_point_capped(temperature),
            software_armed,
        }
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn lat_lon(&self) -> (f64, f64) {
        self.lat_lon
    }

    pub fn battery_v(&self) -> f32 {
        factories::BatteryV::to_float(self.battery_v)
    }

    pub fn temperature(&self) -> f32 {
        factories::Temperature::to_float(self.temperature)
    }

    pub fn software_armed(&self) -> bool {
        self.software_armed
    }
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct PadDiagnosticTelemetryPacket {
    pub timestamp: f64,
    pub unix_clock_ready: bool,
    pub lat_lon: Option<(f64, f64)>,
    pub battery_v: u16,
    pub temperature: u16,
    pub hardware_armed: bool,
    pub software_armed: bool,

    pub num_of_fix_satellites: u8,
    pub pyro_main_continuity: bool,
    pub pyro_drogue_continuity: bool,
    // TODO more
}

impl PadDiagnosticTelemetryPacket {
    pub fn new(
        timestamp: f64,
        unix_clock_ready: bool,
        lat_lon: Option<(f64, f64)>,
        battery_v: f32,
        temperature: f32,
        hardware_armed: bool,
        software_armed: bool,
        num_of_fix_satellites: u8,
        pyro_main_continuity: bool,
        pyro_drogue_continuity: bool,
    ) -> Self {
        Self {
            timestamp,
            unix_clock_ready,
            lat_lon,
            battery_v: factories::BatteryV::to_fixed_point_capped(battery_v),
            temperature: factories::Temperature::to_fixed_point_capped(temperature),
            hardware_armed,
            software_armed,
            num_of_fix_satellites,
            pyro_main_continuity,
            pyro_drogue_continuity,
        }
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn unix_clock_ready(&self) -> bool {
        self.unix_clock_ready
    }

    pub fn lat_lon(&self) -> Option<(f64, f64)> {
        self.lat_lon
    }

    pub fn battery_v(&self) -> f32 {
        factories::BatteryV::to_float(self.battery_v)
    }

    pub fn temperature(&self) -> f32 {
        factories::Temperature::to_float(self.temperature)
    }

    pub fn hardware_armed(&self) -> bool {
        self.hardware_armed
    }

    pub fn software_armed(&self) -> bool {
        self.software_armed
    }

    pub fn num_of_fix_satellites(&self) -> u8 {
        self.num_of_fix_satellites
    }

    pub fn pyro_main_continuity(&self) -> bool {
        self.pyro_main_continuity
    }

    pub fn pyro_drogue_continuity(&self) -> bool {
        self.pyro_drogue_continuity
    }
}

#[derive(defmt::Format, Debug, Clone, Copy, Archive, Deserialize, Serialize, PartialEq, Eq)]
pub enum FlightCoreStateTelemetry {
    DisArmed,
    Armed,
    PowerAscend,
    Coast,
    Descent,
    Landed,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct InFlightTelemetryPacket {
    pub timestamp: f64,
    pub temperature: u16,
    pub altitude: u16,
    pub max_altitude: u16,
    pub max_speed: u16,
    pub lat_lon: (f64, f64),
    pub battery_v: u16,
    pub flight_core_state: FlightCoreStateTelemetry,
}

impl InFlightTelemetryPacket{
    pub fn new(
        timestamp: f64,
        temperature: f32,
        altitude: f32,
        max_altitude: f32,
        max_speed: f32,
        lat_lon: (f64, f64),
        battery_v: f32,
        flight_core_state: FlightCoreStateTelemetry,
    ) -> Self {
        Self {
            timestamp,
            temperature: factories::Temperature::to_fixed_point_capped(temperature),
            altitude: factories::Altitude::to_fixed_point_capped(altitude),
            max_altitude: factories::Altitude::to_fixed_point_capped(max_altitude),
            max_speed: factories::Altitude::to_fixed_point_capped(max_speed),
            lat_lon,
            battery_v: factories::BatteryV::to_fixed_point_capped(battery_v),
            flight_core_state,
        }
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn temperature(&self) -> f32 {
        factories::Temperature::to_float(self.temperature)
    }

    pub fn altitude(&self) -> f32 {
        factories::Altitude::to_float(self.altitude)
    }

    pub fn max_altitude(&self) -> f32 {
        factories::Altitude::to_float(self.max_altitude)
    }

    pub fn max_speed(&self) -> f32 {
        factories::Altitude::to_float(self.max_speed)
    }

    pub fn lat_lon(&self) -> (f64, f64) {
        self.lat_lon
    }

    pub fn battery_v(&self) -> f32 {
        factories::BatteryV::to_float(self.battery_v)
    }

    pub fn flight_core_state(&self) -> FlightCoreStateTelemetry {
        self.flight_core_state
    }

}

pub struct TelemetryPacketBuilderState {
    pub gps_location: Option<GPSLocation>,
    pub battery_v: f32,
    pub temperature: f32,
    pub altitude: f32,
    max_altitude: f32,
    pub speed: f32,
    max_speed: f32,

    pub hardware_armed: bool,
    pub software_armed: bool,
    pub pyro_main_continuity: bool,
    pub pyro_drogue_continuity: bool,
    pub flight_core_state: FlightCoreStateTelemetry,
}

pub struct TelemetryPacketBuilder<'a, K: Clock> {
    unix_clock: UnixClock<'a, K>,
    state: BlockingMutex<NoopRawMutex, RefCell<TelemetryPacketBuilderState>>,
}

impl<'a, K: Clock> TelemetryPacketBuilder<'a, K> {
    pub fn new(unix_clock: UnixClock<'a, K>) -> Self {
        Self {
            unix_clock,
            state: BlockingMutex::new(RefCell::new(TelemetryPacketBuilderState {
                gps_location: None,
                battery_v: 0.0,
                temperature: 0.0,
                altitude: 0.0,
                max_altitude: 0.0,
                speed: 0.0,
                max_speed: 0.0,
                hardware_armed: false,
                software_armed: false,
                pyro_main_continuity: false,
                pyro_drogue_continuity: false,
                flight_core_state: FlightCoreStateTelemetry::DisArmed,
            })),
        }
    }

    fn is_pad_ready(&self) -> bool {
        if !self.unix_clock.ready() {
            return false;
        }
        self.state.lock(|state| {
            let state = state.borrow();

            if let Some(gps_location) = &state.gps_location {
                if gps_location.lat_lon.is_none() {
                    return false;
                }
            } else {
                return false;
            }
            if state.hardware_armed {
                return false;
            }
            if state.pyro_main_continuity {
                return false;
            }
            if state.pyro_drogue_continuity {
                return false;
            }

            return true;
        })
    }

    pub fn create_packet(&self) -> VLPDownlinkPacket {
        let is_pad_ready = self.is_pad_ready();
        self.state.lock(|state| {
            let state = state.borrow();

            if state.flight_core_state == FlightCoreStateTelemetry::DisArmed
                || state.flight_core_state == FlightCoreStateTelemetry::Armed
            {
                // create pad telemetry packet
                if is_pad_ready {
                    PadIdleTelemetryPacket::new(
                        self.unix_clock.now_ms(),
                        state.gps_location.as_ref().unwrap().lat_lon.unwrap(),
                        state.battery_v,
                        state.temperature,
                        state.software_armed,
                    )
                    .into()
                } else {
                    PadDiagnosticTelemetryPacket::new(
                        self.unix_clock.now_ms(),
                        self.unix_clock.ready(),
                        state.gps_location.as_ref().map(|l| l.lat_lon).flatten(),
                        state.battery_v,
                        state.temperature,
                        state.hardware_armed,
                        state.software_armed,
                        state
                            .gps_location
                            .as_ref()
                            .map_or(0, |l| l.num_of_fix_satellites),
                        state.pyro_main_continuity,
                        state.pyro_drogue_continuity,
                    )
                    .into()
                }
            } else {
                // create in-flight telemetry packet
                todo!()
            }
        })
    }

    pub fn update<U>(&self, update_fn: U)
    where
        U: FnOnce(&mut RefMut<TelemetryPacketBuilderState>) -> (),
    {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            update_fn(&mut state);
            state.max_altitude = state.altitude.max(state.max_altitude);
            state.max_speed = state.speed.max(state.max_speed);
        })
    }
}
