use crate::virt_drivers::{
    buzzer::SpeakerBuzzer, debugger::Debugger, sensors::VirtualIMU, serial::VirtualSerial,
    timer::TokioTimer,
};
use firmware_common::{
    driver::{
        adc::DummyADC,
        arming::DummyHardwareArming,
        barometer::DummyBarometer,
        debugger::Debugger as DebuggerDriver,
        dummy_radio_kind::DummyRadioKind,
        gps::{DummyGPS, DummyGPSCtrl},
        imu::IMU,
        indicator::DummyIndicator,
        meg::DummyMegnetometer,
        pyro::{DummyContinuity, DummyPyroCtrl},
        rng::DummyRNG,
        serial::Serial,
        sys_reset::PanicSysReset,
        usb::DummyUSB,
    },
    init, DeviceManager,
};
use std::{path::PathBuf, sync::{Arc, Barrier}};
use std::thread;
use vlfs::DummyCrc;
use vlfs_host::FileFlash;

pub fn start_avionics_thread(
    flash_file_name: PathBuf,
    imu: VirtualIMU,
    serial: VirtualSerial,
    debugger: Debugger,
    ready_barrier: Arc<Barrier>
) {
    thread::spawn(move || {
        ready_barrier.wait();
        avionics(flash_file_name, imu, serial, debugger);
    });
}

#[tokio::main]
async fn avionics(
    flash_file_name: PathBuf,
    imu: impl IMU,
    serial: impl Serial,
    debugger: impl DebuggerDriver,
) {
    let timer = TokioTimer {};
    let mut device_manager = DeviceManager::new(
        PanicSysReset {},
        timer,
        FileFlash::new(flash_file_name).await.unwrap(),
        DummyCrc {},
        imu,
        DummyADC::new(timer),
        DummyADC::new(timer),
        (DummyContinuity::new(timer), DummyPyroCtrl {}),
        (DummyContinuity::new(timer), DummyPyroCtrl {}),
        (DummyContinuity::new(timer), DummyPyroCtrl {}),
        DummyHardwareArming::new(timer),
        serial,
        DummyUSB::new(timer),
        SpeakerBuzzer::new(),
        DummyMegnetometer::new(timer),
        DummyRadioKind {},
        DummyRNG {},
        DummyIndicator {},
        DummyIndicator {},
        DummyBarometer::new(timer),
        (DummyGPS::new(timer), DummyGPSCtrl {}), // VLF3's GPS doesn't work
        debugger,
    );

    init(&mut device_manager, None).await;
}
