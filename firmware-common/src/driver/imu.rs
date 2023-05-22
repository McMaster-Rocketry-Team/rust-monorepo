pub use ferraris_calibration::IMUReading;

use super::timer::Timer;

pub trait IMU {
    type Error: defmt::Format;

    async fn wait_for_power_on(&mut self) -> Result<(), Self::Error>;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<IMUReading, Self::Error>;
}

pub struct DummyIMU<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyIMU<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}

impl<T: Timer> IMU for DummyIMU<T> {
    type Error = ();

    async fn wait_for_power_on(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading, ()> {
        self.timer.sleep(1.0).await;
        Ok(IMUReading {
            timestamp: 0.0,
            acc: [0.0, 0.0, 0.0],
            gyro: [0.0, 0.0, 0.0],
        })
    }
}
