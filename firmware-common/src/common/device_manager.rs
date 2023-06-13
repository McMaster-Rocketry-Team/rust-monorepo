use defmt::warn;
use lora_phy::{mod_traits::RadioKind, LoRa};
use vlfs::{Crc, Flash};

use crate::driver::{
    adc::ADC,
    arming::HardwareArming,
    barometer::Barometer,
    buzzer::Buzzer,
    debugger::{Debugger, RadioApplicationLayer},
    gps::{GPSCtrl, GPS},
    imu::IMU,
    indicator::Indicator,
    meg::Megnetometer,
    pyro::{Continuity, PyroCtrl},
    rng::RNG,
    serial::Serial,
    sys_reset::SysReset,
    timer::{DelayUsWrapper, Timer},
    usb::USB,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

#[allow(dead_code)]
pub struct DeviceManager<
    DB: Debugger,
    D: SysReset,
    T: Timer,
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
    GT: GPSCtrl,
> {
    pub(crate) sys_reset: Mutex<CriticalSectionRawMutex, D>,
    pub(crate) flash: Mutex<CriticalSectionRawMutex, F>,
    pub(crate) crc: Mutex<CriticalSectionRawMutex, C>,
    pub(crate) imu: Mutex<CriticalSectionRawMutex, I>,
    pub(crate) batt_voltmeter: Mutex<CriticalSectionRawMutex, V>,
    pub(crate) batt_ammeter: Mutex<CriticalSectionRawMutex, A>,
    pub(crate) pyro1_cont: Mutex<CriticalSectionRawMutex, P1C>,
    pub(crate) pyro1_ctrl: Mutex<CriticalSectionRawMutex, P1T>,
    pub(crate) pyro2_cont: Mutex<CriticalSectionRawMutex, P2C>,
    pub(crate) pyro2_ctrl: Mutex<CriticalSectionRawMutex, P2T>,
    pub(crate) pyro3_cont: Mutex<CriticalSectionRawMutex, P3C>,
    pub(crate) pyro3_ctrl: Mutex<CriticalSectionRawMutex, P3T>,
    pub(crate) arming_switch: Mutex<CriticalSectionRawMutex, ARM>,
    pub(crate) serial: Mutex<CriticalSectionRawMutex, S>,
    pub(crate) usb: Mutex<CriticalSectionRawMutex, U>,
    pub(crate) buzzer: Mutex<CriticalSectionRawMutex, B>,
    pub(crate) meg: Mutex<CriticalSectionRawMutex, M>,
    radio_kind: Option<L>,
    pub(crate) lora: Mutex<CriticalSectionRawMutex, Option<LoRa<L>>>,
    pub(crate) rng: Mutex<CriticalSectionRawMutex, R>,
    pub(crate) status_indicator: Mutex<CriticalSectionRawMutex, IS>,
    pub(crate) error_indicator: Mutex<CriticalSectionRawMutex, IE>,
    pub(crate) barometer: Mutex<CriticalSectionRawMutex, BA>,
    pub(crate) gps: Mutex<CriticalSectionRawMutex, G>,
    pub(crate) gps_ctrl: Mutex<CriticalSectionRawMutex, GT>,
    pub(crate) timer: T,
    pub(crate) debugger: DB,
}

impl<
        DB: Debugger,
        D: SysReset,
        T: Timer,
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
        GT: GPSCtrl,
    >
    DeviceManager<
        DB,
        D,
        T,
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
        L,
        R,
        IS,
        IE,
        BA,
        G,
        GT,
    >
{
    pub fn new(
        sys_reset: D,
        timer: T,
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
        radio_kind: L,
        rng: R,
        status_indicator: IS,
        error_indicator: IE,
        barometer: BA,
        gps: (G, GT),
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
            serial: Mutex::new(serial),
            usb: Mutex::new(usb),
            buzzer: Mutex::new(buzzer),
            meg: Mutex::new(meg),
            radio_kind: Some(radio_kind),
            lora: Mutex::new(None),
            rng: Mutex::new(rng),
            status_indicator: Mutex::new(status_indicator),
            error_indicator: Mutex::new(error_indicator),
            barometer: Mutex::new(barometer),
            gps: Mutex::new(gps.0),
            gps_ctrl: Mutex::new(gps.1),
            timer,
        }
    }

    pub async fn init(&mut self) {
        if let Some(radio_kind) = self.radio_kind.take() {
            match LoRa::new(radio_kind, false, &mut DelayUsWrapper(self.timer)).await {
                Ok(lora) => {
                    self.lora.get_mut().replace(lora);
                }
                Err(e) => {
                    warn!("Failed to initialize LoRa: {:?}", e);
                }
            };
        }
    }

    // TODO get application layer from lora
    pub async fn get_radio_application_layer(&self) -> Option<impl RadioApplicationLayer> {
        self.debugger.get_vlp_application_layer()
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
macro_rules! device_manager_type{
    () => { &DeviceManager<
    impl Debugger,
    impl SysReset,
    impl Timer,
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
    impl GPSCtrl,
>};

(mut) => { &mut DeviceManager<
    impl Debugger,
    impl SysReset,
    impl Timer,
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
    impl GPSCtrl,
>}
}

pub mod prelude {
    pub use super::DeviceManager;
    pub use crate::driver::adc::ADC;
    pub use crate::driver::arming::HardwareArming;
    pub use crate::driver::barometer::Barometer;
    pub use crate::driver::buzzer::Buzzer;
    pub use crate::driver::debugger::Debugger;
    pub use crate::driver::gps::{GPSCtrl, GPS};
    pub use crate::driver::imu::IMU;
    pub use crate::driver::indicator::Indicator;
    pub use crate::driver::meg::Megnetometer;
    pub use crate::driver::pyro::{Continuity, PyroCtrl};
    pub use crate::driver::rng::RNG;
    pub use crate::driver::serial::Serial;
    pub use crate::driver::sys_reset::SysReset;
    pub use crate::driver::timer::Timer;
    pub use crate::driver::usb::USB;
    pub use lora_phy::mod_traits::RadioKind;
    pub use lora_phy::LoRa;
    pub use vlfs::{Crc, Flash};
}
