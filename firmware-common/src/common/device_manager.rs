use core::cell::RefCell;

use lora_phy::mod_traits::RadioKind;
use vlfs::{Crc, Flash};

use crate::driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer,
    device_management::DeviceManagement, gps::GPS, imu::IMU, indicator::Indicator,
    meg::Megnetometer, pyro::PyroChannel, rng::RNG, serial::Serial, timer::Timer, usb::USB,
};
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex};
pub struct DeviceManagerInner<
    D: DeviceManagement,
    F: Flash,
    C: Crc,
    I: IMU,
    V: ADC,
    A: ADC,
    P1: PyroChannel,
    P2: PyroChannel,
    P3: PyroChannel,
    ARM: HardwareArming,
    S: Serial,
    U: USB,
    B: Buzzer,
    M: Megnetometer,
    L: RadioKind + 'static,
    R: RNG,
    IS: Indicator,
    IE: Indicator,
    BA: Barometer,
    G: GPS,
> {
    pub device_management: Option<D>,
    pub flash: Option<F>,
    pub crc: Option<C>,
    pub imu: Option<I>,
    pub batt_voltmeter: Option<V>,
    pub batt_ammeter: Option<A>,
    pub pyro1: Option<P1>,
    pub pyro2: Option<P2>,
    pub pyro3: Option<P3>,
    pub arming_switch: Option<ARM>,
    pub serial: Option<S>,
    pub usb: Option<U>,
    pub buzzer: Option<B>,
    pub meg: Option<M>,
    pub radio_kind: Option<L>,
    pub rng: Option<R>,
    pub status_indicator: Option<IS>,
    pub error_indicator: Option<IE>,
    pub barometer: Option<BA>,
    pub gps: Option<G>,
}

pub struct DeviceManager<
    D: DeviceManagement,
    T: Timer,
    F: Flash,
    C: Crc,
    I: IMU,
    V: ADC,
    A: ADC,
    P1: PyroChannel,
    P2: PyroChannel,
    P3: PyroChannel,
    ARM: HardwareArming,
    S: Serial,
    U: USB,
    B: Buzzer,
    M: Megnetometer,
    L: RadioKind + 'static,
    R: RNG,
    IS: Indicator,
    IE: Indicator,
    BA: Barometer,
    G: GPS,
> {
    inner: BlockingMutex<
        CriticalSectionRawMutex,
        RefCell<
            DeviceManagerInner<D, F, C, I, V, A, P1, P2, P3, ARM, S, U, B, M, L, R, IS, IE, BA, G>,
        >,
    >,
    timer: T,
}

impl<
        D: DeviceManagement,
        T: Timer,
        F: Flash,
        C: Crc,
        I: IMU,
        V: ADC,
        A: ADC,
        P1: PyroChannel,
        P2: PyroChannel,
        P3: PyroChannel,
        ARM: HardwareArming,
        S: Serial,
        U: USB,
        B: Buzzer,
        M: Megnetometer,
        L: RadioKind + 'static,
        R: RNG,
        IS: Indicator,
        IE: Indicator,
        BA: Barometer,
        G: GPS,
    > DeviceManager<D, T, F, C, I, V, A, P1, P2, P3, ARM, S, U, B, M, L, R, IS, IE, BA, G>
{
    pub fn new(
        device_management: D,
        timer: T,
        flash: F,
        crc: C,
        imu: I,
        batt_voltmeter: V,
        batt_ammeter: A,
        pyro1: P1,
        pyro2: P2,
        pyro3: P3,
        arming_switch: ARM,
        serial: S,
        usb: U,
        buzzer: B,
        meg: M,
        radio_kind: L,
        rng: R,
        status_indicator: IS,
        error_indicator: IE,
        barometer: BA,
        gps: G,
    ) -> Self {
        Self {
            inner: BlockingMutex::new(RefCell::new(DeviceManagerInner {
                device_management: Some(device_management),
                flash: Some(flash),
                crc: Some(crc),
                imu: Some(imu),
                batt_voltmeter: Some(batt_voltmeter),
                batt_ammeter: Some(batt_ammeter),
                pyro1: Some(pyro1),
                pyro2: Some(pyro2),
                pyro3: Some(pyro3),
                arming_switch: Some(arming_switch),
                serial: Some(serial),
                usb: Some(usb),
                buzzer: Some(buzzer),
                meg: Some(meg),
                radio_kind: Some(radio_kind),
                rng: Some(rng),
                status_indicator: Some(status_indicator),
                error_indicator: Some(error_indicator),
                barometer: Some(barometer),
                gps: Some(gps),
            })),
            timer,
        }
    }

    pub fn lock<RE>(
        &self,
        func: impl FnOnce(
            &mut DeviceManagerInner<
                D,
                F,
                C,
                I,
                V,
                A,
                P1,
                P2,
                P3,
                ARM,
                S,
                U,
                B,
                M,
                L,
                R,
                IS,
                IE,
                BA,
                G,
            >,
        ) -> RE,
    ) -> RE {
        self.inner.lock(|inner| func(&mut *inner.borrow_mut()))
    }

    pub fn timer(&self) -> T {
        self.timer
    }
}

#[macro_export]
macro_rules! claim_devices {
    ($device_manager:ident, $device:ident) => {
        #[allow(unused_mut)]
        let mut $device = $device_manager.lock(|devices| devices.$device.take().unwrap());
    };
    ($device_manager:ident, $($device:ident),+) => {
        #[allow(unused_mut)]
        let ($(mut $device),+) = $device_manager.lock(|devices| ($(
            devices.$device.take().unwrap(),
        )+));
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

#[macro_export]
macro_rules! return_devices {
    ($device_manager:ident, $device:ident) => {
        $device_manager.lock(|devices| {
            devices.$device = Some($device);
            Option::<()>::None
        });
    };
    ($device_manager:ident, $($device:ident),+) => {
        $device_manager.lock(|devices| {
            $(
                devices.$device = Some($device);
            )+
            Option::<()>::None
        });
    };
}

// import all the device types using `crate::common::device_manager::prelude::*;`
#[macro_export]
macro_rules! device_manager_type{
    () => { &DeviceManager<
    impl DeviceManagement,
    impl Timer,
    impl Flash,
    impl Crc,
    impl IMU,
    impl ADC,
    impl ADC,
    impl PyroChannel,
    impl PyroChannel,
    impl PyroChannel,
    impl HardwareArming,
    impl Serial,
    impl USB,
    impl Buzzer,
    impl Megnetometer,
    impl RadioKind + 'static,
    impl RNG,
    impl Indicator,
    impl Indicator,
    impl Barometer,
    impl GPS,
>}
}

pub mod prelude {
    pub use super::DeviceManager;
    pub use crate::driver::adc::ADC;
    pub use crate::driver::arming::HardwareArming;
    pub use crate::driver::barometer::Barometer;
    pub use crate::driver::buzzer::Buzzer;
    pub use crate::driver::device_management::DeviceManagement;
    pub use crate::driver::gps::GPS;
    pub use crate::driver::imu::IMU;
    pub use crate::driver::indicator::Indicator;
    pub use crate::driver::meg::Megnetometer;
    pub use crate::driver::pyro::PyroChannel;
    pub use crate::driver::rng::RNG;
    pub use crate::driver::serial::Serial;
    pub use crate::driver::timer::Timer;
    pub use crate::driver::usb::USB;
    pub use lora_phy::mod_traits::RadioKind;
    pub use vlfs::{Crc, Flash};
}
