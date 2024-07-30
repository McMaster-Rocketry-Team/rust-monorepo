use crate::common::delta_logger::prelude::*;
use crate::common::unix_clock::UnixClock;
use crate::common::variable_int::VariableIntRkyvWrapper;
use crate::driver::gps::GPSData;
use crate::Clock;
use crate::{common::fixed_point::F32FixedPointFactory, fixed_point_factory2};
use core::cell::{RefCell, RefMut};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};
use int_enum::IntEnum;
use packed_struct::prelude::*;
use rkyv::{Archive, Deserialize, Serialize};

fixed_point_factory2!(BatteryVFac, f32, 6.0, 9.0, 0.001);
fixed_point_factory2!(TemperatureFac, f32, -30.0, 85.0, 0.1);
fixed_point_factory2!(FreeSpaceFac, f32, 0.0, 524288.0, 128.0);
fixed_point_factory2!(AltitudeFac, f32, 0.0, 5000.0, 5.0);
fixed_point_factory2!(VerticalSpeedFac, f32, -400.0, 400.0, 2.0);

#[repr(u8)]
#[derive(defmt::Format, Debug, Clone, Copy, IntEnum)]
pub enum FlightCoreStateTelemetry {
    DisArmed = 0,
    Armed = 1,
    PowerAscend = 2,
    Coast = 3,
    Descent = 4,
    Landed = 5,
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct TelemetryPacket {
    unix_clock_ready: bool,
    timestamp: u32, // seconds since unix epoch / seconds since boot

    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    num_of_fix_satellites: Integer<u8, packed_bits::Bits<5>>,
    lat_lon: (f64, f64),

    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    battery_v: BatteryVFacPacked,
    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    temperature: TemperatureFacPacked,
    hardware_armed: bool,
    software_armed: bool,
    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    disk_free_space: FreeSpaceFacPacked,

    pyro_main_continuity: bool,
    pyro_drogue_continuity: bool,

    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    altitude: AltitudeFacPacked,
    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    max_altitude: AltitudeFacPacked,

    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    vertical_speed: VerticalSpeedFacPacked,
    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    max_vertical_speed: VerticalSpeedFacPacked,

    #[defmt(Debug2Format)]
    #[with(VariableIntRkyvWrapper)]
    flight_core_state: Integer<u8, packed_bits::Bits<3>>,
}

impl TelemetryPacket {
    pub fn new(
        unix_clock_ready: bool,
        timestamp: f64,

        num_of_fix_satellites: u8,
        lat_lon: Option<(f64, f64)>,

        battery_v: f32,
        temperature: f32,

        hardware_armed: bool,
        software_armed: bool,

        free_space: u32,

        pyro_main_continuity: bool,
        pyro_drogue_continuity: bool,

        altitude: f32,
        max_altitude: f32,

        vertical_speed: f32,
        max_vertical_speed: f32,

        flight_core_state: FlightCoreStateTelemetry,
    ) -> Self {
        Self {
            unix_clock_ready,
            timestamp: (timestamp / 1000.0) as u32,
            num_of_fix_satellites: num_of_fix_satellites.into(),
            lat_lon: lat_lon.unwrap_or((0.0, 0.0)),
            battery_v: BatteryVFac::to_fixed_point_capped(battery_v),
            temperature: TemperatureFac::to_fixed_point_capped(temperature),
            hardware_armed,
            software_armed,
            disk_free_space: FreeSpaceFac::to_fixed_point_capped(free_space as f32),
            pyro_main_continuity,
            pyro_drogue_continuity,
            altitude: AltitudeFac::to_fixed_point_capped(altitude),
            max_altitude: AltitudeFac::to_fixed_point_capped(max_altitude),
            vertical_speed: VerticalSpeedFac::to_fixed_point_capped(vertical_speed),
            max_vertical_speed: VerticalSpeedFac::to_fixed_point_capped(max_vertical_speed),
            flight_core_state: (flight_core_state as u8).into(),
        }
    }

    pub fn unix_clock_ready(&self) -> bool {
        self.unix_clock_ready
    }

    /// Get the timestamp in milliseconds
    pub fn timestamp(&self) -> f64 {
        self.timestamp as f64 * 1000.0
    }

    pub fn num_of_fix_satellites(&self) -> u8 {
        self.num_of_fix_satellites.into()
    }

    pub fn lat_lon(&self) -> Option<(f64, f64)> {
        if self.lat_lon.0 == 0.0 && self.lat_lon.1 == 0.0 {
            None
        } else {
            Some(self.lat_lon)
        }
    }

    pub fn battery_v(&self) -> f32 {
        BatteryVFac::to_float(self.battery_v)
    }

    pub fn temperature(&self) -> f32 {
        TemperatureFac::to_float(self.temperature)
    }

    pub fn hardware_armed(&self) -> bool {
        self.hardware_armed
    }

    pub fn software_armed(&self) -> bool {
        self.software_armed
    }

    /// Get the free space in bytes
    pub fn free_space(&self) -> f32 {
        FreeSpaceFac::to_float(self.disk_free_space)
    }

    pub fn pyro_main_continuity(&self) -> bool {
        self.pyro_main_continuity
    }

    pub fn pyro_drogue_continuity(&self) -> bool {
        self.pyro_drogue_continuity
    }

    pub fn altitude(&self) -> f32 {
        AltitudeFac::to_float(self.altitude)
    }

    pub fn max_altitude(&self) -> f32 {
        AltitudeFac::to_float(self.max_altitude)
    }

    pub fn vertical_speed(&self) -> f32 {
        VerticalSpeedFac::to_float(self.vertical_speed)
    }

    pub fn max_vertical_speed(&self) -> f32 {
        VerticalSpeedFac::to_float(self.max_vertical_speed)
    }

    pub fn flight_core_state(&self) -> FlightCoreStateTelemetry {
        let flight_core_state: u8 = self.flight_core_state.into();
        if let Ok(flight_core_state) = FlightCoreStateTelemetry::try_from(flight_core_state) {
            flight_core_state
        } else {
            FlightCoreStateTelemetry::DisArmed
        }
    }
}

impl BitArraySerializable for TelemetryPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.unix_clock_ready);
        writer.write(self.timestamp);
        writer.write(self.num_of_fix_satellites);
        writer.write(self.lat_lon);
        writer.write(self.battery_v);
        writer.write(self.temperature);
        writer.write(self.hardware_armed);
        writer.write(self.software_armed);
        writer.write(self.disk_free_space);
        writer.write(self.pyro_main_continuity);
        writer.write(self.pyro_drogue_continuity);
        writer.write(self.altitude);
        writer.write(self.max_altitude);
        writer.write(self.vertical_speed);
        writer.write(self.max_vertical_speed);
        writer.write(self.flight_core_state);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            unix_clock_ready: reader.read().unwrap(),
            timestamp: reader.read().unwrap(),
            num_of_fix_satellites: reader.read().unwrap(),
            lat_lon: reader.read().unwrap(),
            battery_v: reader.read().unwrap(),
            temperature: reader.read().unwrap(),
            hardware_armed: reader.read().unwrap(),
            software_armed: reader.read().unwrap(),
            disk_free_space: reader.read().unwrap(),
            pyro_main_continuity: reader.read().unwrap(),
            pyro_drogue_continuity: reader.read().unwrap(),
            altitude: reader.read().unwrap(),
            max_altitude: reader.read().unwrap(),
            vertical_speed: reader.read().unwrap(),
            max_vertical_speed: reader.read().unwrap(),
            flight_core_state: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        bool::len_bits()
            + u32::len_bits()
            + <Integer<u8, packed_bits::Bits<5>>>::len_bits()
            + <(f64, f64)>::len_bits()
            + BatteryVFacPacked::len_bits()
            + TemperatureFacPacked::len_bits()
            + bool::len_bits()
            + bool::len_bits()
            + FreeSpaceFacPacked::len_bits()
            + bool::len_bits()
            + bool::len_bits()
            + AltitudeFacPacked::len_bits()
            + AltitudeFacPacked::len_bits()
            + VerticalSpeedFacPacked::len_bits()
            + VerticalSpeedFacPacked::len_bits()
            + <Integer<u8, packed_bits::Bits<3>>>::len_bits()
    }
}

pub struct TelemetryPacketBuilderState {
    pub gps_location: Option<GPSData>,
    pub battery_v: f32,
    pub temperature: f32,
    pub altitude: f32,
    max_altitude: f32,
    pub vertical_speed: f32,
    max_vertical_speed: f32,

    pub hardware_armed: bool,
    pub software_armed: bool,
    pub pyro_main_continuity: bool,
    pub pyro_drogue_continuity: bool,
    pub flight_core_state: FlightCoreStateTelemetry,
    pub disk_free_space: u32,
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
                vertical_speed: 0.0,
                max_vertical_speed: 0.0,
                hardware_armed: false,
                software_armed: false,
                pyro_main_continuity: false,
                pyro_drogue_continuity: false,
                flight_core_state: FlightCoreStateTelemetry::DisArmed,
                disk_free_space: 0,
            })),
        }
    }

    pub fn create_packet(&self) -> TelemetryPacket {
        self.state.lock(|state| {
            let state = state.borrow();

            TelemetryPacket::new(
                self.unix_clock.ready(),
                self.unix_clock.now_ms(),
                state
                    .gps_location
                    .as_ref()
                    .map_or(0, |l| l.num_of_fix_satellites),
                state.gps_location.as_ref().map(|l| l.lat_lon).flatten(),
                state.battery_v,
                state.temperature,
                state.hardware_armed,
                state.software_armed,
                state.disk_free_space,
                state.pyro_main_continuity,
                state.pyro_drogue_continuity,
                state.altitude,
                state.max_altitude,
                state.vertical_speed,
                state.max_vertical_speed,
                state.flight_core_state,
            )
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
            state.max_vertical_speed = state.vertical_speed.max(state.max_vertical_speed);
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn print_telemetry_packet_length() {
        println!("Telemetry Packet Length: {}", TelemetryPacket::len_bits());
    }
}
