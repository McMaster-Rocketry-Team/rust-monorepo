#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

use common::console::console::Console;
use defmt::*;
use driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer, gps::GPS, imu::IMU,
    indicator::Indicator, lora::LoRa, meg::Megnetometer, pyro::PyroChannel, rng::RNG,
    serial::Serial, timer::Timer,
};
use futures::future::join3;
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
    B: Buzzer,
    M: Megnetometer,
    // L: LoRa,
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
    mut serial: S,
    mut buzzer: B,
    mut meg: M,
    // mut lora: L,
    mut rng: R,
    mut status_indicator: IS,
    mut error_indicator: IE,
    mut barometer: BA,
    mut gps: G,
) {
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);
    gps.reset().await;

    // let mut usb_buffer = [0u8; 64];
    // timer.sleep(2000).await;

    let indicator_fut = async {
        loop {
            timer.sleep(2000).await;
            status_indicator.set_enable(true).await;
            timer.sleep(10).await;
            status_indicator.set_enable(false).await;
        }
    };

    let imu_fut = async {
        // loop {
        //     let _ = imu.read().await;
        // }
    };

    // barometer.reset().await.unwrap();
    // let baro_fut = async {
    //     loop {
    //         timer.sleep(1000).await;
    //         info!("baro: {}", barometer.read().await);
    //     }
    // };

    let mut console = Console::new(timer, serial, fs, pyro3, buzzer);
    join3(console.run(), indicator_fut, imu_fut).await;

    return;

    // meg.reset(false).await.unwrap();
    let mut time = timer.now_micros();
    loop {
        let _ = imu.read().await;
        let new_time = timer.now_micros();
        // info!(
        //     "{}Hz",
        //     (1.0 / (((new_time - time) as f64) / 1000.0 / 1000.0)) as u32
        // );
        time = new_time;
        // info!(
        //     "battery: {}V, {}A",
        //     batt_voltmeter.read().await,
        //     batt_ammeter.read().await
        // );
        // info!("arming: {}", arming_switch.read_arming().await);
        // info!("pyro1 cont: {}", pyro1.read_continuity().await);
        // buzzer.set_enable(true).await;
        // timer.sleep(500).await;
        // info!("meg: {}", meg.read().await);
        // timer.sleep(500).await;
        // let len = usb.read_packet(&mut usb_buffer).await;
        // if let Ok(len) = len {
        //     info!("usb: {}", &usb_buffer[0..len]);
        // }
    }

    // count down 5s, log every 1s
    // for i in 0..5 {
    //     info!(
    //         "arming: {}  chan 3 cont: {}",
    //         arming_switch.read_arming().await,
    //         pyro3.read_continuity().await
    //     );
    //     info!(
    //         "voltage: {}  current: {}",
    //         batt_voltmeter.read().await,
    //         batt_ammeter.read().await
    //     );
    //     info!("countdown: {}", 5 - i);
    //     timer.sleep(1000).await;
    //

    // loop {
    //     pyro3.set_enable(true).await;
    //     timer.sleep(1).await;
    //     pyro3.set_enable(false).await;
    //     timer.sleep(15).await;
    // }
}
