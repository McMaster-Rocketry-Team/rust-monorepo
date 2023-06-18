use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use serde::Serialize;
use std::{
    fs::File,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{ArmingState, AvionicsMarker};

#[derive(Debug, Serialize)]
struct OrientationData {
    timestamp: f64,
    q_w: f32,
    q_x: f32,
    q_y: f32,
    q_z: f32,
    x: f32,
    y: f32,
    z: f32,
    speed_x: f32,
    speed_y: f32,
    speed_z: f32,
    gyr_x: f32,
    gyr_y: f32,
    gyr_z: f32,
}

#[derive(Component)]
pub struct OrientationTx(Arc<Mutex<Sender<OrientationData>>>);

impl OrientationTx {
    fn new(tx: Sender<OrientationData>) -> Self {
        Self(Arc::new(Mutex::new(tx)))
    }

    fn send(&self, data: OrientationData) {
        let sender = self.0.lock().unwrap();
        sender.send(data).unwrap();
    }
}

pub fn setup_orientation_logger(mut commands: Commands) {
    let (tx, rx) = channel::<OrientationData>();

    commands.spawn(OrientationTx::new(tx));
    start_orientation_writer_thread(rx);
}

pub fn orientation_logger_system(
    (av_state, orientation_tx): (
        Query<(&Velocity, &Transform), With<AvionicsMarker>>,
        Query<&OrientationTx>,
    ),
    arming_state: ResMut<State<ArmingState>>,
) {
    let arming_state = arming_state.0;
    let orientation_tx = orientation_tx.iter().next().unwrap();
    let (velocity, transform) = av_state.iter().next().unwrap();
    if arming_state == ArmingState::Armed {
        orientation_tx.send(OrientationData {
            timestamp: now_mills(),
            q_w: transform.rotation.w,
            q_x: transform.rotation.x,
            q_y: transform.rotation.y,
            q_z: transform.rotation.z,
            x: transform.translation.x,
            y: transform.translation.y,
            z: transform.translation.z,
            speed_x: velocity.linvel.x,
            speed_y: velocity.linvel.y,
            speed_z: velocity.linvel.z,
            gyr_x: velocity.angvel.x.to_degrees(),
            gyr_y: velocity.angvel.y.to_degrees(),
            gyr_z: velocity.angvel.z.to_degrees(),
        })
    }
}

fn start_orientation_writer_thread(rx: Receiver<OrientationData>) {
    thread::spawn(move || {
        let file = File::create(format!("simulation-{}.csv", now_mills())).unwrap();
        let mut wtr = csv::Writer::from_writer(file);
        loop {
            let data = rx.recv().unwrap();
            wtr.serialize(data).unwrap();
        }
    });
}

fn now_mills() -> f64 {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs_f64() * 1000.0
}
