use std::cell::RefCell;

use embassy_sync::{
    blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex},
    signal::Signal,
};
use firmware_common::driver::{
    arming::HardwareArming,
    barometer::{BaroReading, Barometer},
    gps::{GPSLocation, GPS},
    imu::{IMUReading, IMU},
    indicator::Indicator,
    mag::{MagReading, Magnetometer},
    pyro::{Continuity, PyroCtrl},
    timestamp::BootTimestamp,
};

pub struct VirtualBaro {
    signal: Signal<CriticalSectionRawMutex, BaroReading<BootTimestamp>>,
}

impl VirtualBaro {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    pub fn set_reading(&self, reading: BaroReading<BootTimestamp>) {
        self.signal.signal(reading);
    }
}

impl Barometer for &VirtualBaro {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self) -> Result<BaroReading<BootTimestamp>, Self::Error> {
        Ok(self.signal.try_take().expect("Baro reading not set"))
    }
}

pub struct VirtualIMU {
    signal: Signal<CriticalSectionRawMutex, IMUReading<BootTimestamp>>,
}

impl VirtualIMU {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    pub fn set_reading(&self, reading: IMUReading<BootTimestamp>) {
        self.signal.signal(reading);
    }
}

impl IMU for &VirtualIMU {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading<BootTimestamp>, Self::Error> {
        Ok(self.signal.try_take().expect("IMU reading not set"))
    }
}

pub struct VirtualMag {
    signal: Signal<CriticalSectionRawMutex, MagReading<BootTimestamp>>,
}

impl VirtualMag {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    pub fn set_reading(&self, reading: MagReading<BootTimestamp>) {
        self.signal.signal(reading);
    }
}

impl Magnetometer for &VirtualMag {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self) -> Result<MagReading<BootTimestamp>, Self::Error> {
        Ok(self.signal.try_take().expect("Mag reading not set"))
    }
}

pub struct VirtualGPS {
    signal: Signal<CriticalSectionRawMutex, GPSLocation>,
}

impl VirtualGPS {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    pub fn set_reading(&self, reading: GPSLocation) {
        self.signal.signal(reading);
    }
}

impl GPS for &VirtualGPS {
    type Error = ();

    async fn next_location(&mut self) -> Result<GPSLocation, Self::Error> {
        Ok(self.signal.try_take().expect("GPS reading not set"))
    }
}

pub struct VirtualArmingSwitch {
    signal: Signal<CriticalSectionRawMutex, bool>,
    armed: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl VirtualArmingSwitch {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
            armed: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub fn set_armed(&self, new_armed: bool) {
        self.armed.lock(|armed| {
            if *armed.borrow() != new_armed {
                *armed.borrow_mut() = new_armed;
                self.signal.signal(new_armed);
            }
        });
    }
}

impl HardwareArming for &VirtualArmingSwitch {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, Self::Error> {
        Ok(self.signal.wait().await)
    }

    async fn read_arming(&mut self) -> Result<bool, Self::Error> {
        Ok(self.armed.lock(|armed| *armed.borrow()))
    }
}

pub struct VirtualContinuity {
    signal: Signal<CriticalSectionRawMutex, bool>,
    cont: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl VirtualContinuity {
    pub fn new() -> Self {
        Self {
            signal: Signal::new(),
            cont: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub fn set_continuity(&self, continuity: bool) {
        self.cont.lock(|cont| {
            if *cont.borrow() != continuity {
                *cont.borrow_mut() = continuity;
                self.signal.signal(continuity);
            }
        });
    }
}

impl Continuity for &VirtualContinuity {
    type Error = ();

    async fn wait_continuity_change(&mut self) -> Result<bool, Self::Error> {
        Ok(self.signal.wait().await)
    }

    async fn read_continuity(&mut self) -> Result<bool, Self::Error> {
        Ok(self.cont.lock(|cont| *cont.borrow()))
    }
}

pub struct VirtualPyroCtrl {
    enabled: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl VirtualPyroCtrl {
    pub fn new() -> Self {
        Self {
            enabled: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.lock(|enabled| *enabled.borrow())
    }
}

impl PyroCtrl for &VirtualPyroCtrl {
    type Error = ();

    async fn set_enable(&mut self, enable: bool) -> Result<(), Self::Error> {
        self.enabled.lock(|enabled| *enabled.borrow_mut() = enable);
        Ok(())
    }
}

pub struct VirtualIndicator {
    enabled: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl VirtualIndicator {
    pub fn new() -> Self {
        Self {
            enabled: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.lock(|enabled| *enabled.borrow())
    }
}

impl Indicator for &VirtualIndicator {
    async fn set_enable(&mut self, enable: bool) {
        self.enabled.lock(|enabled| *enabled.borrow_mut() = enable);
    }
}
