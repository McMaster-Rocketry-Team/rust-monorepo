#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

use common::console::console::Console;
use defmt::*;
use driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer, gps::GPS, imu::IMU,
    indicator::Indicator, meg::Megnetometer, pyro::PyroChannel, rng::RNG, serial::Serial,
    timer::Timer, usb::USB,
};
use futures::future::join3;
use lora_phy::mod_traits::RadioKind;
use vlfs::{Crc, Flash, VLFS};

mod avionics;
mod common;
pub mod driver;
mod gcm;
mod utils;

pub async fn init<
    T: Timer,
    F: Flash + defmt::Format,
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
    L: RadioKind,
    R: RNG,
    IS: Indicator,
    IE: Indicator,
    BA: Barometer,
    G: GPS,
>(
    timer: T,
    mut flash: F,
    crc: C,
    _imu: I,
    _batt_voltmeter: V,
    _batt_ammeter: A,
    _pyro1: P1,
    _pyro2: P2,
    _pyro3: P3,
    _arming_switch: ARM,
    serial: S,
    mut usb: U,
    _buzzer: B,
    _meg: M,
    _lora: L,
    _rng: R,
    mut status_indicator: IS,
    _error_indicator: IE,
    _barometer: BA,
    mut gps: G,
) -> ! {
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    gps.reset().await;

    let indicator_fut = async {
        loop {
            timer.sleep(2000).await;
            status_indicator.set_enable(true).await;
            timer.sleep(10).await;
            status_indicator.set_enable(false).await;
        }
    };

    let mut console1 = Console::new(timer, serial, &fs);
    let mut console2 = Console::new(timer, usb, &fs);

    join3(console1.run(), console2.run(), indicator_fut).await;

    defmt::panic!("wtf");
}
