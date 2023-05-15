use defmt::warn;
use lora_phy::{mod_traits::RadioKind, LoRa};
use vlfs::{Crc, Flash};

use crate::driver::{
    adc::ADC,
    arming::HardwareArming,
    barometer::Barometer,
    buzzer::Buzzer,
    gps::GPS,
    imu::IMU,
    indicator::Indicator,
    meg::Megnetometer,
    pyro::PyroChannel,
    rng::RNG,
    serial::Serial,
    sys_reset::SysReset,
    timer::{DelayUsWrapper, Timer},
    usb::USB,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

#[allow(dead_code)]
pub struct DeviceManager<
    D: SysReset,
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
    pub(crate) device_management: Mutex<CriticalSectionRawMutex, D>,
    pub(crate) flash: Mutex<CriticalSectionRawMutex, F>,
    pub(crate) crc: Mutex<CriticalSectionRawMutex, C>,
    pub(crate) imu: Mutex<CriticalSectionRawMutex, I>,
    pub(crate) batt_voltmeter: Mutex<CriticalSectionRawMutex, V>,
    pub(crate) batt_ammeter: Mutex<CriticalSectionRawMutex, A>,
    pub(crate) pyro1: Mutex<CriticalSectionRawMutex, P1>,
    pub(crate) pyro2: Mutex<CriticalSectionRawMutex, P2>,
    pub(crate) pyro3: Mutex<CriticalSectionRawMutex, P3>,
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
    pub(crate) timer: T,
}

impl<
        D: SysReset,
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
            device_management: Mutex::new(device_management),
            flash: Mutex::new(flash),
            crc: Mutex::new(crc),
            imu: Mutex::new(imu),
            batt_voltmeter: Mutex::new(batt_voltmeter),
            batt_ammeter: Mutex::new(batt_ammeter),
            pyro1: Mutex::new(pyro1),
            pyro2: Mutex::new(pyro2),
            pyro3: Mutex::new(pyro3),
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
            gps: Mutex::new(gps),
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
    impl SysReset,
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
>};

(mut) => { &mut DeviceManager<
    impl SysReset,
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
    pub use crate::driver::gps::GPS;
    pub use crate::driver::imu::IMU;
    pub use crate::driver::indicator::Indicator;
    pub use crate::driver::meg::Megnetometer;
    pub use crate::driver::pyro::PyroChannel;
    pub use crate::driver::rng::RNG;
    pub use crate::driver::serial::Serial;
    pub use crate::driver::sys_reset::SysReset;
    pub use crate::driver::timer::Timer;
    pub use crate::driver::usb::USB;
    pub use lora_phy::mod_traits::RadioKind;
    pub use lora_phy::LoRa;
    pub use vlfs::{Crc, Flash};
}
