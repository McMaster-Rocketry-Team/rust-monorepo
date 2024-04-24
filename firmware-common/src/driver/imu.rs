use core::fmt::Debug;

pub use ferraris_calibration::IMUReading;

use embedded_hal_async::delay::DelayNs;

pub trait IMU {
    type Error: defmt::Format + Debug;

    async fn wait_for_power_on(&mut self) -> Result<(), Self::Error>;
    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<IMUReading, Self::Error>;
}

pub struct DummyIMU<D: DelayNs> {
    delay: D,
}

impl<D: DelayNs> DummyIMU<D> {
    pub fn new(delay: D) -> Self {
        Self { delay }
    }
}

impl<D: DelayNs> IMU for DummyIMU<D> {
    type Error = ();

    async fn wait_for_power_on(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<IMUReading, ()> {
        self.delay.delay_ms(1).await;
        Ok(IMUReading {
            timestamp: 0.0,
            acc: [0.0, 0.0, 0.0],
            gyro: [0.0, 0.0, 0.0],
        })
    }
}
