#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(let_chains)]

use common::{console::console::Console, rwlock::rwLockTest};
use defmt::info;
use driver::{
    adc::ADC, arming::HardwareArming, buzzer::Buzzer, crc::Crc, flash::SpiFlash, imu::IMU,
    lora::LoRa, meg::Megnetometer, pyro::PyroChannel, rng::RNG, timer::Timer,
};

use crate::common::vlfs::VLFS;

mod avionics;
mod common;
pub mod driver;
mod gcm;
mod utils;

pub async fn init<
    T: Timer,
    F: SpiFlash,
    C: Crc,
    I: IMU,
    V: ADC,
    A: ADC,
    P1: PyroChannel,
    P2: PyroChannel,
    P3: PyroChannel,
    ARM: HardwareArming,
    USB: driver::usb::USB,
    B: Buzzer,
    M: Megnetometer,
    // L: LoRa,
    R: RNG,
>(
    timer: T,
    flash: F,
    crc: C,
    mut imu: I,
    mut batt_voltmeter: V,
    mut batt_ammeter: A,
    mut pyro1: P1,
    mut pyro2: P2,
    mut pyro3: P3,
    mut arming_switch: ARM,
    mut usb: USB,
    mut buzzer: B,
    mut meg: M,
    // mut lora: L,
    mut rng: R,
) {
    let mut fs = VLFS::new(flash, crc);
    fs.init().await;
    // let mut usb_buffer = [0u8; 64];
    // timer.sleep(2000).await;

    let mut console = Console::new(timer, usb, fs, pyro3);
    console.run().await.unwrap();

    // meg.reset(false).await.unwrap();
    // loop {
    // let reading = imu.read().await;
    // info!("imu: {}", reading);
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
    // }

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
