#![cfg_attr(not(test), no_std)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]

use common::console::console::Console;
use defmt::*;
use driver::{
    adc::ADC, arming::HardwareArming, barometer::Barometer, buzzer::Buzzer,
    device_management::DeviceManagement, gps::GPS, imu::IMU, indicator::Indicator,
    meg::Megnetometer, pyro::PyroChannel, rng::RNG, serial::Serial, timer::Timer, usb::USB,
};
use futures::{
    future::{join5, select},
    pin_mut,
};
use lora_phy::mod_traits::RadioKind;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    beacon::{beacon_receiver::beacon_receiver, beacon_sender::beacon_sender},
    common::{
        device_mode::{read_device_mode, write_device_mode, DeviceMode},
        gps_parser::GPSParser,
    },
};

mod allocator;
mod avionics;
mod beacon;
mod common;
pub mod driver;
mod gcm;
mod utils;

pub async fn init<
    D: DeviceManagement,
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
    L: RadioKind + 'static,
    R: RNG,
    IS: Indicator,
    IE: Indicator,
    BA: Barometer,
    G: GPS,
>(
    device_management: D,
    timer: T,
    mut flash: F,
    crc: C,
    mut imu: I,
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
    radio_kind: L,
    _rng: R,
    status_indicator: IS,
    error_indicator: IE,
    _barometer: BA,
    gps: G,
) -> ! {
    flash.reset().await.ok();
    let mut fs = VLFS::new(flash, crc);
    unwrap!(fs.init().await);

    let imu_fut = async {
        imu.reset().await.ok();
        loop {
            let _ = imu.read().await;
        }
    };

    let indicator_fut = async {
        // loop {
        //     timer.sleep(2000).await;
        //     status_indicator.set_enable(true).await;
        //     timer.sleep(10).await;
        //     status_indicator.set_enable(false).await;
        // }
    };

    let usb_connected = {
        let timeout_fut = timer.sleep(500);
        let usb_wait_connection_fut = usb.wait_connection();
        pin_mut!(timeout_fut);
        pin_mut!(usb_wait_connection_fut);
        match select(timeout_fut, usb_wait_connection_fut).await {
            futures::future::Either::Left(_) => {
                info!("USB not connected");
                false
            }
            futures::future::Either::Right(_) => {
                info!("USB connected");
                true
            }
        }
    };

    let mut serial_console = Console::new(timer, serial, &fs, device_management);
    let mut usb_console = Console::new(timer, usb, &fs, device_management);

    let main_fut = async {
        if usb_connected {
            info!("USB connected on boot, stopping main");
            return;
        }

        let mut device_mode = DeviceMode::Avionics;
        if let Some(device_mode_) = read_device_mode(&fs).await {
            info!("Read device mode from disk: {}", device_mode_);
            device_mode = device_mode_;
        } else {
            info!("No device mode file found, creating one");
            try_or_warn!(write_device_mode(&fs, device_mode).await);
        }

        info!("Starting in mode {}", device_mode);
        match device_mode {
            DeviceMode::Avionics => defmt::panic!("Avionics mode not implemented"),
            DeviceMode::GCM => defmt::panic!("GCM mode not implemented"),
            DeviceMode::BeaconSender => {
                beacon_sender(
                    timer,
                    &fs,
                    GPSParser::new(gps),
                    radio_kind,
                    status_indicator,
                    error_indicator,
                )
                .await
            }
            DeviceMode::BeaconReceiver => beacon_receiver(timer, &fs, radio_kind).await,
        };
    };

    join5(
        imu_fut,
        main_fut,
        serial_console.run(),
        usb_console.run(),
        indicator_fut,
    )
    .await;

    defmt::panic!("wtf");
}
