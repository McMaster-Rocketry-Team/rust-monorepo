use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use firmware_common::driver::{
    barometer::{BaroReading, Barometer}, imu::{IMUReading, IMU}, mag::{MagReading, Magnetometer}, timestamp::BootTimestamp
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
