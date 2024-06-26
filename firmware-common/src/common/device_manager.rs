use heapless::Vec;
use lora_phy::{mod_traits::RadioKind, LoRa};
use vlfs::{Crc, Flash, VLFS};

use crate::driver::{
    adc::{Ampere, Volt, ADC},
    arming::HardwareArming,
    barometer::Barometer,
    buzzer::Buzzer,
    camera::Camera,
    can_bus::SplitableCanBus,
    clock::Clock,
    debugger::Debugger,
    delay::Delay,
    gps::{GPSLocation, GPS, GPSPPS},
    imu::IMU,
    indicator::Indicator,
    mag::Magnetometer,
    pyro::{Continuity, PyroCtrl},
    rng::RNG,
    serial::SplitableSerial,
    sys_reset::SysReset,
    usb::SplitableUSB,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex, pubsub::{PubSubChannel, Subscriber}};

use super::{
    buzzer_queue::BuzzerQueue, indicator_controller::IndicatorController, unix_clock::UnixClock,
};

#[allow(dead_code)]
pub struct DeviceManager<
    DB: Debugger,
    D: SysReset,
    T: Clock,
    DL: Delay,
    F: Flash,
    C: Crc,
    I: IMU,
    IH: IMU,
    V: ADC<Volt>,
    A: ADC<Ampere>,
    PC: Continuity,
    PT: PyroCtrl,
    ARM: HardwareArming,
    S: SplitableSerial,
    U: SplitableUSB,
    B: Buzzer,
    M: Magnetometer,
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
    pub(crate) low_g_imu: Mutex<NoopRawMutex, I>,
    pub(crate) high_g_imu: Mutex<NoopRawMutex, IH>,
    pub(crate) batt_voltmeter: Mutex<NoopRawMutex, V>,
    pub(crate) batt_ammeter: Mutex<NoopRawMutex, A>,
    pub(crate) pyro1_cont: Mutex<NoopRawMutex, PC>,
    pub(crate) pyro1_ctrl: Mutex<NoopRawMutex, PT>,
    pub(crate) pyro2_cont: Mutex<NoopRawMutex, PC>,
    pub(crate) pyro2_ctrl: Mutex<NoopRawMutex, PT>,
    pub(crate) pyro3_cont: Mutex<NoopRawMutex, Option<PC>>,
    pub(crate) pyro3_ctrl: Mutex<NoopRawMutex, Option<PT>>,
    pub(crate) arming_switch: Mutex<NoopRawMutex, ARM>,
    pub(crate) serial: Mutex<NoopRawMutex, Option<S>>,
    pub(crate) usb: Mutex<NoopRawMutex, Option<U>>,
    pub(crate) buzzer: Mutex<NoopRawMutex, B>,
    pub(crate) mag: Mutex<NoopRawMutex, M>,
    pub(crate) lora: Mutex<NoopRawMutex, LoRa<RK, DL>>,
    pub(crate) rng: Mutex<NoopRawMutex, R>,
    pub(crate) indicators: Mutex<NoopRawMutex, IndicatorController<IR, IG, IB, T, DL>>,
    pub(crate) barometer: Mutex<NoopRawMutex, BA>,
    pub(crate) gps: Mutex<NoopRawMutex, G>,
    pub(crate) gps_pps: Mutex<NoopRawMutex, GP>,
    pub(crate) camera: Mutex<NoopRawMutex, CAM>,
    pub(crate) can_bus: Mutex<NoopRawMutex, CB>,
    pub(crate) can_bus_health_check_ids: Vec<u32, 8>,
    pub(crate) clock: T,
    pub(crate) delay: DL,
    pub(crate) debugger: DB,
}

impl<
        DB: Debugger,
        D: SysReset,
        T: Clock,
        DL: Delay,
        F: Flash,
        C: Crc,
        I: IMU,
        IH: IMU,
        V: ADC<Volt>,
        A: ADC<Ampere>,
        PC: Continuity,
        PT: PyroCtrl,
        ARM: HardwareArming,
        S: SplitableSerial,
        U: SplitableUSB,
        B: Buzzer,
        M: Magnetometer,
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
        IH,
        V,
        A,
        PC,
        PT,
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
        low_g_imu: I,
        high_g_imu: IH,
        batt_voltmeter: V,
        batt_ammeter: A,
        pyro1: (PC, PT),
        pyro2: (PC, PT),
        pyro3: Option<(PC, PT)>,
        arming_switch: ARM,
        serial: S,
        usb: U,
        buzzer: B,
        mag: M,
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
        can_bus_health_check_ids: Vec<u32, 8>,
        debugger: DB,
    ) -> Self {
        let (pyro3_cont, pyro3_ctrl) = if pyro3.is_some() {
            let pyro3 = pyro3.unwrap();
            (Some(pyro3.0), Some(pyro3.1))
        } else {
            (None, None)
        };
        Self {
            debugger,
            sys_reset: Mutex::new(sys_reset),
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
            low_g_imu: Mutex::new(low_g_imu),
            high_g_imu: Mutex::new(high_g_imu),
            batt_voltmeter: Mutex::new(batt_voltmeter),
            batt_ammeter: Mutex::new(batt_ammeter),
            pyro1_cont: Mutex::new(pyro1.0),
            pyro1_ctrl: Mutex::new(pyro1.1),
            pyro2_cont: Mutex::new(pyro2.0),
            pyro2_ctrl: Mutex::new(pyro2.1),
            pyro3_cont: Mutex::new(pyro3_cont),
            pyro3_ctrl: Mutex::new(pyro3_ctrl),
            arming_switch: Mutex::new(arming_switch),
            serial: Mutex::new(Some(serial)),
            usb: Mutex::new(Some(usb)),
            buzzer: Mutex::new(buzzer),
            mag: Mutex::new(mag),
            lora: Mutex::new(lora),
            rng: Mutex::new(rng),
            indicators: Mutex::new(IndicatorController::new(
                red_indicator,
                green_indicator,
                blue_indicator,
                clock.clone(),
                delay.clone(),
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

    pub fn delay(&self) -> DL {
        self.delay.clone()
    }

    pub fn clock(&self) -> T {
        self.clock.clone()
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
    impl Delay,
    impl Flash,
    impl Crc,
    impl IMU,
    impl IMU,
    impl ADC<Volt>,
    impl ADC<Ampere>,
    impl Continuity,
    impl PyroCtrl,
    impl HardwareArming,
    impl SplitableSerial,
    impl SplitableUSB,
    impl Buzzer,
    impl Magnetometer,
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
    impl Delay,
    impl Flash,
    impl Crc,
    impl IMU,
    impl IMU,
    impl ADC<Volt>,
    impl ADC<Ampere>,
    impl Continuity,
    impl PyroCtrl,
    impl HardwareArming,
    impl SplitableSerial,
    impl SplitableUSB,
    impl Buzzer,
    impl Magnetometer,
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
    pub use crate::driver::adc::{Ampere, Volt, ADC};
    pub use crate::driver::arming::HardwareArming;
    pub use crate::driver::barometer::Barometer;
    pub use crate::driver::buzzer::Buzzer;
    pub use crate::driver::camera::Camera;
    pub use crate::driver::can_bus::SplitableCanBus;
    pub use crate::driver::clock::Clock;
    pub use crate::driver::debugger::Debugger;
    pub use crate::driver::delay::Delay;
    pub use crate::driver::gps::{GPS, GPSPPS};
    pub use crate::driver::imu::IMU;
    pub use crate::driver::indicator::Indicator;
    pub use crate::driver::mag::Magnetometer;
    pub use crate::driver::pyro::{Continuity, PyroCtrl};
    pub use crate::driver::radio::RadioPhy;
    pub use crate::driver::rng::RNG;
    pub use crate::driver::serial::SplitableSerial;
    pub use crate::driver::sys_reset::SysReset;
    pub use crate::driver::usb::SplitableUSB;
    pub use crate::system_services_type;
    pub use lora_phy::mod_traits::RadioKind;
    pub use vlfs::{Crc, Flash};
}

pub struct SystemServices<'f, 'a, 'b, 'c, DL: Delay, T: Clock, F: Flash, C: Crc> {
    pub(crate) fs: &'f VLFS<F, C>,
    pub(crate) gps: &'a PubSubChannel::<NoopRawMutex, GPSLocation, 1, 1, 1>,
    pub(crate) delay: DL,
    pub(crate) clock: T,
    pub(crate) unix_clock: UnixClock<'b, T>,
    pub(crate) buzzer_queue: BuzzerQueue<'c>,
}

impl<'f, 'a, 'b, 'c, DL: Delay, T: Clock, F: Flash, C: Crc>
    SystemServices<'f, 'a, 'b, 'c, DL, T, F, C>
{
    pub fn delay(&self) -> DL {
        self.delay.clone()
    }

    pub fn clock(&self) -> T {
        self.clock.clone()
    }

    pub fn unix_clock(&self) -> UnixClock<'b, T> {
        self.unix_clock.clone()
    }
}

#[macro_export]
macro_rules! system_services_type {
    () => { &SystemServices<
        '_,
        '_,
        '_,
        '_,
        impl Delay,
        impl Clock,
        impl Flash,
        impl Crc,
    >};
}
