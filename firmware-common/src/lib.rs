#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]

use defmt::info;
use driver::{
    adc::ADC, arming::HardwareArming, buzzer::Buzzer, crc::Crc, flash::SpiFlash, imu::IMU,
    meg::Megnetometer, pyro::PyroChannel, timer::Timer,
};

use crate::common::vlfs::VLFS;

mod avionics;
mod common;
pub mod driver;
mod gcm;

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
) {
    let mut fs = VLFS::new(flash, crc);
    fs.init().await;
    let mut usb_buffer = [0u8; 64];


    meg.reset(false).await.unwrap();
    loop {
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
        info!("meg: {}", meg.read().await);
        timer.sleep(500).await;
        // let len = usb.read_packet(&mut usb_buffer).await;
        // if let Ok(len) = len {
        //     info!("usb: {}", &usb_buffer[0..len]);
        // }
    }
}
