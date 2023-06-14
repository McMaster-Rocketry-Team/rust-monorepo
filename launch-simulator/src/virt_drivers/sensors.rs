use std::{
    f32::consts::PI,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::log::info;
use bevy::prelude::{Component, Transform};
use bevy_rapier3d::prelude::Velocity;
use firmware_common::driver::{
    barometer::{BaroReading, Barometer},
    imu::{IMUReading, IMU},
};
use nalgebra::{UnitQuaternion, Vector3};
use rand;
use rand_distr::{Distribution, Normal};
use tokio::sync::watch::{self, Receiver, Sender};
#[derive(Component)]
pub struct SensorSender {
    tx: Sender<SensorSnapshot>,
    last_state: Option<(f64, Velocity, Transform)>,
    ready: bool,
    acc_normal: Normal<f32>,
    gyro_normal: Normal<f32>,
}

impl SensorSender {
    pub fn new(tx: Sender<SensorSnapshot>) -> Self {
        Self {
            tx,
            last_state: None,
            ready: false,
            acc_normal: Normal::new(0.0, 0.3).unwrap(),
            gyro_normal: Normal::new(0.0, 0.1).unwrap(),
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
            (new_velocity - last_velocity) / ((now - self.last_state.unwrap().0) / 1000.0) as f32;

        let gyro = velocity.angvel;

        let mut rng = rand::thread_rng();

        // these are world frame
        let acc = Vector3::new(
            acceleration.x + self.acc_normal.sample(&mut rng),
            acceleration.y + self.acc_normal.sample(&mut rng) + 9.81,
            acceleration.z + self.acc_normal.sample(&mut rng),
        );

        let gyro = Vector3::new(
            gyro.x / PI * 180.0 + self.gyro_normal.sample(&mut rng),
            gyro.y / PI * 180.0 + self.gyro_normal.sample(&mut rng),
            gyro.z / PI * 180.0 + self.gyro_normal.sample(&mut rng),
        );

        // convert to local frame
        let quat: UnitQuaternion<f32> = transform.rotation.into();
        let quat = quat.inverse();

        let acc = quat * acc;
        let gyro = quat * gyro;

        // println!("acc: {} | gyro: {}", format_vec(acc), format_vec(gyro));

        let imu_reading = IMUReading {
            timestamp: now,
            acc: acc.into(),
            gyro: gyro.into(),
        };

        let baro_reading = BaroReading {
            timestamp: now,
            pressure: baro_imposter(transform.translation.y, 50.0),
            temperature: 50.0,
        };

        self.tx
            .send(SensorSnapshot {
                imu_reading,
                baro_reading,
            })
            .unwrap();

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
    baro_reading: BaroReading,
}

pub fn create_sensors() -> (SensorSender, VirtualIMU, VirtualBaro) {
    let (tx, rx) = watch::channel(SensorSnapshot::default());

    (
        SensorSender::new(tx),
        VirtualIMU { rx: rx.clone() },
        VirtualBaro { rx },
    )
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

pub struct VirtualBaro {
    rx: Receiver<SensorSnapshot>,
}

impl Barometer for VirtualBaro {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn read(&mut self) -> Result<BaroReading, Self::Error> {
        Ok(self.rx.borrow().baro_reading.clone())
    }
}

fn format_float(num: f32) -> String {
    let sign = if num.is_sign_positive() { " " } else { "-" };
    let abs_num = num.abs();
    let int_part = abs_num.trunc() as u32;
    let frac_part = ((abs_num - int_part as f32) * 100.0).round() as u32;

    format!("{}{:02}.{:02}", sign, int_part, frac_part)
}

fn format_vec(vec: Vector3<f32>) -> String {
    format!(
        "[{}, {}, {}]",
        format_float(vec.x),
        format_float(vec.y),
        format_float(vec.z)
    )
}

fn baro_imposter(altitude: f32, temperature: f32) -> f32 {
    const SEA_LEVEL_PRESSURE: f32 = 101325.0;

    SEA_LEVEL_PRESSURE * (1.0 - (0.0065 * altitude) / (temperature + 273.15)).powf(5.255)
}
