use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::{Component, Transform};
use bevy_rapier3d::prelude::Velocity;
use firmware_common::driver::imu::{IMUReading, IMU};
use tokio::sync::watch::{self, Receiver, Sender};

#[derive(Component)]
pub struct SensorSender {
    tx: Sender<SensorSnapshot>,
    last_state: Option<(f64, Velocity, Transform)>,
    ready: bool,
}

impl SensorSender {
    pub fn new(tx: Sender<SensorSnapshot>) -> Self {
        Self {
            tx,
            last_state: None,
            ready: false,
        }
    }

    fn now_mills(&self) -> f64 {
        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
        since_the_epoch.as_secs_f64() * 1000.0
    }

    pub fn update_state(&mut self, velocity: Velocity, transform: Transform) {
        if self.last_state.is_none() {
            self.last_state = Some((self.now_mills(), velocity, transform));
            return;
        }

        let now = self.now_mills();
        let last_velocity = self.last_state.unwrap().1.linvel;
        let new_velocity = velocity.linvel;
        let acceleration =
            (new_velocity - last_velocity) / (now - self.last_state.unwrap().0) as f32;

        let gyro = velocity.angvel;

        let imu_reading = IMUReading {
            timestamp: now,
            acc: acceleration.into(),
            gyro: gyro.into(),
        };

        self.tx.send(SensorSnapshot { imu_reading }).unwrap();

        self.last_state = Some((now, velocity, transform));
        self.ready = true;
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

#[derive(Clone, Default, Debug)]
pub struct SensorSnapshot {
    imu_reading: IMUReading,
}

pub fn create_sensors() -> (SensorSender, VirtualIMU) {
    let (tx, rx) = watch::channel(SensorSnapshot::default());

    (SensorSender::new(tx), VirtualIMU { rx })
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
