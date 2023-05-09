#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

use common::console::console::Console;
use defmt::*;
use driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer, gps::GPS, imu::IMU,
    indicator::Indicator, meg::Megnetometer, pyro::PyroChannel, rng::RNG, serial::Serial,
    timer::Timer,
};
use futures::future::{join, join3};
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
    S1: Serial,
    S2: Serial,
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
    mut imu: I,
    mut batt_voltmeter: V,
    mut batt_ammeter: A,
    mut pyro1: P1,
    mut pyro2: P2,
    mut pyro3: P3,
    mut arming_switch: ARM,
    mut serial1: S1,
    mut serial2: S2,
    mut buzzer: B,
    mut meg: M,
    mut lora: L,
    mut rng: R,
    mut status_indicator: IS,
    mut error_indicator: IE,
    mut barometer: BA,
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

    let mut console1 = Console::new(timer, serial1, &fs);
    let mut console2 = Console::new(timer, serial2, &fs);

    join3(console1.run(), console2.run(), indicator_fut).await;
    // join(console1.run(), indicator_fut).await;

    defmt::panic!("wtf");
}
