#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use std::sync::{Arc, Barrier};

use avionics::start_avionics_thread;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::prelude::*;
use firmware_common::driver::debugger::DebuggerEvent;
use keyframe::AnimationPlayer;
use virt_drivers::{
    debugger::{create_debugger, DebuggerReceiver},
    sensors::{create_sensors, SensorSender},
    serial::{create_virtual_serial, VirtualSerial},
};

mod avionics;
mod calibration_system;
mod keyframe;
mod virt_drivers;

pub const AVIONICS_X_LEN: f32 = 0.04;
pub const AVIONICS_Y_LEN: f32 = 0.02;
pub const AVIONICS_Z_LEN: f32 = 0.05;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Launch Simulator".into(),
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(EguiPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(PanOrbitCameraPlugin)
        .add_event::<DebuggerEvent>()
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_startup_system(setup_virtual_avionics)
        .add_system(ui_system)
        .add_system(virtual_sensors)
        .add_system(debugger_receiver)
        .add_system(calibration_system::calibration_system)
        .add_system(avionics_animation_system)
        .run();
}

fn setup_graphics(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-0.3, 0.15, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera::default(),
    ));
}

fn setup_physics(mut commands: Commands) {
    // floor
    commands
        .spawn(Collider::cuboid(5.0, 0.1, 5.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -0.1, 0.0)));

    // avionics
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Sleeping {
            linear_threshold: -1.0,
            angular_threshold: -1.0,
            sleeping: false,
        })
        .insert(Velocity {
            linvel: Vec3::ZERO,
            angvel: Vec3::ZERO,
        })
        .insert(Collider::cuboid(
            AVIONICS_X_LEN / 2.0,
            AVIONICS_Y_LEN / 2.0,
            AVIONICS_Z_LEN / 2.0,
        ))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.1, 0.0)))
        .insert(AvionicsMarker);

    // avionics mount
    commands
        .spawn(RigidBody::KinematicPositionBased)
        .insert(Sleeping {
            linear_threshold: -1.0,
            angular_threshold: -1.0,
            sleeping: false,
        })
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)));
}

fn setup_virtual_avionics(mut commands: Commands) {
    let (debugger, debugger_rx) = create_debugger();
    let (serial_a, serial_b) = create_virtual_serial();
    let (sensor_tx, imu) = create_sensors();
    let ready_barrier = Arc::new(Barrier::new(2));

    commands.spawn(debugger_rx);
    commands.spawn(serial_b);
    commands.spawn(sensor_tx);
    commands.spawn(ReadyBarrier(Some(ready_barrier.clone())));

    start_avionics_thread(
        "./launch-simulator/avionics.fl".into(),
        imu,
        serial_a,
        debugger,
        ready_barrier,
    );
}

fn ui_system(mut contexts: EguiContexts, mut serial: Query<&mut VirtualSerial>) {
    let mut serial = serial.iter_mut().next().unwrap();
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {
        if ui.button("Calibrate").clicked() {
            println!("calibrate");
            serial.blocking_write(&[0, 0, 0, 0, 0, 0, 0, 5]);
        }
        if ui.button("Setup Rocket").clicked() {
            
        }
    });
}

fn virtual_sensors(
    (state, mut ready_barrier, mut sensor_tx): (
        Query<(&Velocity, &Transform), With<AvionicsMarker>>,
        Query<&mut ReadyBarrier>,
        Query<&mut SensorSender>,
    ),
) {
    let (vel, pos) = state.iter().next().unwrap();

    let mut sensor_tx = sensor_tx.iter_mut().next().unwrap();
    sensor_tx.update_state(*vel, *pos);

    let mut ready_barrier = ready_barrier.iter_mut().next().unwrap();
    if sensor_tx.is_ready() && ready_barrier.0.is_some() {
        ready_barrier.0.take().unwrap().wait();
    }
}

fn debugger_receiver(
    mut debugger_receiver: Query<&mut DebuggerReceiver>,
    mut ev_debugger: EventWriter<DebuggerEvent>,
) {
    let mut debugger_receiver = debugger_receiver.iter_mut().next().unwrap();
    while let Some(event) = debugger_receiver.try_recv() {
        println!("debugger: {:?}", event);
        ev_debugger.send(event);
    }
}

fn avionics_animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut animated_entity: Query<(Entity, &mut AnimationPlayer, &mut Transform)>,
) {
    for (entity, mut animation_player, mut transform) in animated_entity.iter_mut() {
        if let Some((orientation, translation)) = animation_player.update(time.delta_seconds()) {
            transform.rotation = orientation.into();
            transform.translation = translation.into();
        } else {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct AvionicsMarker;

#[derive(Component)]
struct ReadyBarrier(Option<Arc<Barrier>>);
