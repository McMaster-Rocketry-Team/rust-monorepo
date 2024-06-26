use crate::virt_drivers::{
    arming::VirtualHardwareArming,
    buzzer::SpeakerBuzzer,
    debugger::Debugger,
    pyro::VirtualPyro,
    sensors::{VirtualBaro, VirtualIMU},
    serial::VirtualSerial,
    timer::TokioTimer,
    vlp_phy::MockPhyParticipant,
};
use firmware_common::{
    driver::{
        adc::DummyADC,
        arming::HardwareArming,
        barometer::Barometer,
        camera::DummyCamera,
        debugger::Debugger as DebuggerDriver,
        gps::{DummyGPS, DummyGPSCtrl},
        imu::IMU,
        indicator::DummyIndicator,
        meg::DummyMegnetometer,
        pyro::{DummyContinuity, DummyPyroCtrl, PyroCtrl},
        rng::DummyRNG,
        serial::Serial,
        sys_reset::PanicSysReset,
        usb::DummyUSB,
    },
    init,
    vlp::phy::VLPPhy,
    DeviceManager, DeviceMode,
};
use std::thread;
use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};
use vlfs::DummyCrc;
use vlfs_host::FileFlash;

pub fn start_avionics_thread(
    flash_file_name: PathBuf,
    imu: VirtualIMU,
    baro: VirtualBaro,
    serial: VirtualSerial,
    debugger: Debugger,
    vlp_phy: MockPhyParticipant,
    arming: VirtualHardwareArming,
    pyro_1: VirtualPyro,
    pyro_2: VirtualPyro,
    mode: Option<DeviceMode>,
    ready_barrier: Arc<Barrier>,
) {
    thread::spawn(move || {
        ready_barrier.wait();
        avionics(
            flash_file_name,
            imu,
            baro,
            serial,
            vlp_phy,
            arming,
            pyro_1,
            pyro_2,
            debugger,
            mode,
        );
    });
}

#[tokio::main]
async fn avionics(
    flash_file_name: PathBuf,
    imu: impl IMU,
    baro: impl Barometer,
    serial: impl Serial,
    vlp_phy: impl VLPPhy,
    arming: impl HardwareArming,
    pyro_1: impl PyroCtrl,
    pyro_2: impl PyroCtrl,
    debugger: impl DebuggerDriver,
    mode: Option<DeviceMode>,
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
        (DummyContinuity::new(timer), pyro_1),
        (DummyContinuity::new(timer), pyro_2),
        (DummyContinuity::new(timer), DummyPyroCtrl {}),
        arming,
        serial,
        DummyUSB::new(timer),
        SpeakerBuzzer::new(),
        DummyMegnetometer::new(timer),
        vlp_phy,
        DummyRNG {},
        DummyIndicator {},
        DummyIndicator {},
        baro,
        (DummyGPS::new(timer), DummyGPSCtrl {}), // VLF3's GPS doesn't work
        DummyCamera {},
        debugger,
    );

    init(&mut device_manager, mode).await;
}
