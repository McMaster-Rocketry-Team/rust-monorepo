use embedded_hal_async::delay::DelayNs;
use lora_phy::{mod_traits::RadioKind, LoRa};
use vlfs::{Crc, Flash, VLFS};
use heapless::Vec;

use crate::{driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer, camera::Camera, can_bus::SplitableCanBus, clock::Clock, debugger::Debugger, gps::{GPS, GPSPPS}, imu::IMU, indicator::Indicator, meg::Megnetometer, pyro::{Continuity, PyroCtrl}, rng::RNG, serial::SplitableSerial, sys_reset::SysReset, usb::SplitableUSB
}, ConfigFiles};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

use super::{
    buzzer_queue::BuzzerQueue, gps_parser::GPSParser, indicator_controller::IndicatorController,
    unix_clock::UnixClock,
};

#[allow(dead_code)]
pub struct DeviceManager<
    DB: Debugger,
    D: SysReset,
    T: Clock,
    DL: DelayNs + Copy,
    F: Flash,
    C: Crc,
    I: IMU,
    V: ADC,
    A: ADC,
    P1C: Continuity,
    P1T: PyroCtrl,
    P2C: Continuity,
    P2T: PyroCtrl,
    P3C: Continuity,
    P3T: PyroCtrl,
    ARM: HardwareArming,
    S: SplitableSerial,
    U: SplitableUSB,
    B: Buzzer,
    M: Megnetometer,
    RK: RadioKind,
    R: RNG,
    IR: Indicator,
    IG: Indicator,
    IB: Indicator,
    BA: Barometer,
    G: GPS,
    GP: GPSPPS,
    CAM: Camera,
    CB: SplitableCanBus,
> {
    pub(crate) sys_reset: Mutex<NoopRawMutex, D>,
    pub(crate) flash: Mutex<NoopRawMutex, F>,
    pub(crate) crc: Mutex<NoopRawMutex, C>,
    pub(crate) imu: Mutex<NoopRawMutex, I>,
    pub(crate) batt_voltmeter: Mutex<NoopRawMutex, V>,
    pub(crate) batt_ammeter: Mutex<NoopRawMutex, A>,
    pub(crate) pyro1_cont: Mutex<NoopRawMutex, P1C>,
    pub(crate) pyro1_ctrl: Mutex<NoopRawMutex, P1T>,
    pub(crate) pyro2_cont: Mutex<NoopRawMutex, P2C>,
    pub(crate) pyro2_ctrl: Mutex<NoopRawMutex, P2T>,
    pub(crate) pyro3_cont: Mutex<NoopRawMutex, P3C>,
    pub(crate) pyro3_ctrl: Mutex<NoopRawMutex, P3T>,
    pub(crate) arming_switch: Mutex<NoopRawMutex, ARM>,
    pub(crate) serial: Mutex<NoopRawMutex, Option<S>>,
    pub(crate) usb: Mutex<NoopRawMutex, Option<U>>,
    pub(crate) buzzer: Mutex<NoopRawMutex, B>,
    pub(crate) meg: Mutex<NoopRawMutex, M>,
    pub(crate) lora: Mutex<NoopRawMutex, LoRa<RK, DL>>,
    pub(crate) rng: Mutex<NoopRawMutex, R>,
    pub(crate) indicators: Mutex<NoopRawMutex, IndicatorController<IR, IG, IB, T, DL>>,
    pub(crate) barometer: Mutex<NoopRawMutex, BA>,
    pub(crate) gps: Mutex<NoopRawMutex, G>,
    pub(crate) gps_pps: Mutex<NoopRawMutex, GP>,
    pub(crate) camera: Mutex<NoopRawMutex, CAM>,
    pub(crate) can_bus: Mutex<NoopRawMutex, CB>,
    pub(crate) can_bus_health_check_ids:Vec<u32,8>,
    pub(crate) clock: T,
    pub(crate) delay: DL,
    pub(crate) debugger: DB,
}

impl<
        DB: Debugger,
        D: SysReset,
        T: Clock,
        DL: DelayNs + Copy,
        F: Flash,
        C: Crc,
        I: IMU,
        V: ADC,
        A: ADC,
        P1C: Continuity,
        P1T: PyroCtrl,
        P2C: Continuity,
        P2T: PyroCtrl,
        P3C: Continuity,
        P3T: PyroCtrl,
        ARM: HardwareArming,
        S: SplitableSerial,
        U: SplitableUSB,
        B: Buzzer,
        M: Megnetometer,
        RK: RadioKind,
        R: RNG,
        IR: Indicator,
        IG: Indicator,
        IB: Indicator,
        BA: Barometer,
        G: GPS,
        GP: GPSPPS,
        CAM: Camera,
        CB: SplitableCanBus,
    >
    DeviceManager<
        DB,
        D,
        T,
        DL,
        F,
        C,
        I,
        V,
        A,
        P1C,
        P1T,
        P2C,
        P2T,
        P3C,
        P3T,
        ARM,
        S,
        U,
        B,
        M,
        RK,
        R,
        IR,
        IG,
        IB,
        BA,
        G,
        GP,
        CAM,
        CB,
    >
{
    pub fn new(
        sys_reset: D,
        clock: T,
        delay: DL,
        flash: F,
        crc: C,
        imu: I,
        batt_voltmeter: V,
        batt_ammeter: A,
        pyro1: (P1C, P1T),
        pyro2: (P2C, P2T),
        pyro3: (P3C, P3T),
        arming_switch: ARM,
        serial: S,
        usb: U,
        buzzer: B,
        meg: M,
        lora: LoRa<RK, DL>,
        rng: R,
        red_indicator: IR,
        green_indicator: IG,
        blue_indicator: IB,
        barometer: BA,
        gps: G,
        gps_pps: GP,
        camera: CAM,
        can_bus: CB,
        can_bus_health_check_ids:Vec<u32,8>,
        debugger: DB,
    ) -> Self {
        Self {
            debugger,
            sys_reset: Mutex::new(sys_reset),
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
            imu: Mutex::new(imu),
            batt_voltmeter: Mutex::new(batt_voltmeter),
            batt_ammeter: Mutex::new(batt_ammeter),
            pyro1_cont: Mutex::new(pyro1.0),
            pyro1_ctrl: Mutex::new(pyro1.1),
            pyro2_cont: Mutex::new(pyro2.0),
            pyro2_ctrl: Mutex::new(pyro2.1),
            pyro3_cont: Mutex::new(pyro3.0),
            pyro3_ctrl: Mutex::new(pyro3.1),
            arming_switch: Mutex::new(arming_switch),
            serial: Mutex::new(Some(serial)),
            usb: Mutex::new(Some(usb)),
            buzzer: Mutex::new(buzzer),
            meg: Mutex::new(meg),
            lora: Mutex::new(lora),
            rng: Mutex::new(rng),
            indicators: Mutex::new(IndicatorController::new(
                red_indicator,
                green_indicator,
                blue_indicator,
                clock,
                delay,
            )),
            barometer: Mutex::new(barometer),
            gps: Mutex::new(gps),
            gps_pps: Mutex::new(gps_pps),
            camera: Mutex::new(camera),
            can_bus: Mutex::new(can_bus),
            can_bus_health_check_ids,
            clock,
            delay,
        }
    }
}

#[macro_export]
macro_rules! claim_devices {
    ($device_manager:ident, $($device:ident),+) => {
        $(
            #[allow(unused_mut)]
            let mut $device = $device_manager.$device.try_lock().unwrap();
        )+
    };
}

#[macro_export]
macro_rules! try_claim_devices {
    ($device_manager:ident, $device:ident) => {{
        #[allow(unused_mut)]
        $device_manager.lock(|devices| devices.$device.take())
    }};
    ($device_manager:ident, $($device:ident),+) => {{
        #[allow(unused_mut)]
        $device_manager.lock(|devices| Some(($(
            devices.$device.take()?,
        )+)))
    }};
}

// import all the device types using `crate::common::device_manager::prelude::*;`
#[macro_export]
macro_rules! device_manager_type {
    () => { &DeviceManager<
    impl Debugger,
    impl SysReset,
    impl Clock,
    impl embedded_hal_async::delay::DelayNs + Copy,
    impl Flash,
    impl Crc,
    impl IMU,
    impl ADC,
    impl ADC,
    impl Continuity,
    impl PyroCtrl,
    impl Continuity,
    impl PyroCtrl,
    impl Continuity,
    impl PyroCtrl,
    impl HardwareArming,
    impl SplitableSerial,
    impl SplitableUSB,
    impl Buzzer,
    impl Megnetometer,
    impl RadioKind,
    impl RNG,
    impl Indicator,
    impl Indicator,
    impl Indicator,
    impl Barometer,
    impl GPS,
    impl GPSPPS,
    impl Camera,
    impl SplitableCanBus,
>};

(mut) => { &mut DeviceManager<
    impl Debugger,
    impl SysReset,
    impl Clock,
    impl embedded_hal_async::delay::DelayNs + Copy,
    impl Flash,
    impl Crc,
    impl IMU,
    impl ADC,
    impl ADC,
    impl Continuity,
    impl PyroCtrl,
    impl Continuity,
    impl PyroCtrl,
    impl Continuity,
    impl PyroCtrl,
    impl HardwareArming,
    impl SplitableSerial,
    impl SplitableUSB,
    impl Buzzer,
    impl Megnetometer,
    impl RadioKind,
    impl RNG,
    impl Indicator,
    impl Indicator,
    impl Indicator,
    impl Barometer,
    impl GPS,
    impl GPSPPS,
    impl Camera,
    impl SplitableCanBus,
>}
}

pub mod prelude {
    pub use super::DeviceManager;
    pub use super::SystemServices;
    pub use crate::device_manager_type;
    pub use crate::driver::adc::ADC;
    pub use crate::driver::arming::HardwareArming;
    pub use crate::driver::barometer::Barometer;
    pub use crate::driver::buzzer::Buzzer;
    pub use crate::driver::camera::Camera;
    pub use crate::driver::clock::Clock;
    pub use crate::driver::debugger::Debugger;
    pub use crate::driver::gps::{GPS, GPSPPS};
    pub use crate::driver::imu::IMU;
    pub use crate::driver::indicator::Indicator;
    pub use crate::driver::meg::Megnetometer;
    pub use crate::driver::pyro::{Continuity, PyroCtrl};
    pub use crate::driver::radio::RadioPhy;
    pub use crate::driver::rng::RNG;
    pub use crate::driver::serial::SplitableSerial;
    pub use crate::driver::sys_reset::SysReset;
    pub use crate::driver::usb::SplitableUSB;
    pub use crate::driver::can_bus::SplitableCanBus;
    pub use crate::common::config_file::ConfigFiles;
    pub use crate::system_services_type;
    pub use embedded_hal_async::delay::DelayNs;
    pub use lora_phy::mod_traits::RadioKind;
    pub use vlfs::{Crc, Flash};
}

pub struct SystemServices<'f, 'a, 'b, 'c, DL: DelayNs + Copy, T: Clock, F: Flash, C: Crc> {
    pub(crate) fs: &'f VLFS<F, C>,
    pub(crate) config: ConfigFiles<'f, F, C>,
    pub(crate) gps: &'a GPSParser,
    pub(crate) delay: DL,
    pub(crate) clock: T,
    pub(crate) unix_clock: UnixClock<'b, T>,
    pub(crate) buzzer_queue: BuzzerQueue<'c>,
}

#[macro_export]
macro_rules! system_services_type {
    () => { &SystemServices<
        '_,
        '_,
        '_,
        '_,
        impl DelayNs + Copy,
        impl Clock,
        impl Flash,
        impl Crc,
    >};
}
