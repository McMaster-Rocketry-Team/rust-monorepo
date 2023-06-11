use firmware_common::driver::imu::{IMUReading, IMU};
use tokio::sync::watch::{self, Receiver, Sender};

#[derive(Clone, Default)]
pub struct SensorSnapshot {
    imu_reading: IMUReading,
}

pub fn create_sensors() -> (Sender<SensorSnapshot>, VirtualIMU) {
    let (tx, rx) = watch::channel(SensorSnapshot::default());

    (tx, VirtualIMU { rx })
}

pub struct VirtualIMU {
    rx: Receiver<SensorSnapshot>,
}

impl IMU for VirtualIMU {
    type Error = ();

    async fn wait_for_power_on(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading, Self::Error> {
        Ok(self.rx.borrow().imu_reading.clone())
    }
}
