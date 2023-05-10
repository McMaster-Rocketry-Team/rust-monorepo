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
use futures::future::join4;
use lora_phy::mod_traits::RadioKind;
use vlfs::{
    io_traits::{AsyncReader, AsyncWriter},
    Crc, Flash, VLFSError, VLFS,
};

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
    usb: U,
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

    let main_fut = async {
        let mut mode = Mode::Avionics;
        if let Some(mode_) = read_mode(&fs).await {
            info!("Read mode from disk: {}", mode_);
            mode = mode_;
        } else {
            info!("No mode file found, creating one");
            try_or_warn!(write_mode(&fs, mode).await);
        }

        info!("Starting in mode {}", mode);
        // match mode_file.mode {
        //     Mode::Avionics => defmt::panic!("Avionics mode not implemented"),
        //     Mode::GCM => defmt::panic!("GCM mode not implemented"),
        //     Mode::BeaconSender => defmt::panic!("Beacon sender mode not implemented"),
        //     Mode::BeaconReceiver => defmt::panic!("Beacon receiver mode not implemented"),
        // }
    };

    join4(main_fut, console1.run(), console2.run(), indicator_fut).await;

    defmt::panic!("wtf");
}

#[derive(Clone, Copy, defmt::Format)]
#[repr(u8)]
enum Mode {
    Avionics = 1,
    GCM = 2,
    BeaconSender = 3,
    BeaconReceiver = 4,
}

impl TryFrom<u8> for Mode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Mode::Avionics),
            2 => Ok(Mode::GCM),
            3 => Ok(Mode::BeaconSender),
            4 => Ok(Mode::BeaconReceiver),
            _ => Err(()),
        }
    }
}

static MODE_FILE_ID: u64 = 0;
static MODE_FILE_TYPE: u16 = 0;

async fn read_mode(fs: &VLFS<impl Flash, impl Crc>) -> Option<Mode> {
    if let Ok(mut reader) = fs.open_file_for_read(MODE_FILE_ID).await {
        let mut file_content = [0u8; 1];
        let read_result = reader.read_all(&mut file_content).await;
        reader.close().await;
        if let Ok(file_content) = read_result {
            if file_content.len() != 1 {
                return None;
            }
            return file_content[0].try_into().ok();
        }
    }
    None
}

async fn write_mode<F: Flash>(fs: &VLFS<F, impl Crc>, mode: Mode) -> Result<(), VLFSError<F>> {
    if fs.exists(MODE_FILE_ID).await {
        fs.remove_file(MODE_FILE_ID).await?;
    }

    fs.create_file(MODE_FILE_ID, MODE_FILE_TYPE).await?;
    let mut writer = fs.open_file_for_write(MODE_FILE_ID).await?;
    writer.extend_from_slice(&[mode as u8]).await?;
    writer.close().await?;
    Ok(())
}
