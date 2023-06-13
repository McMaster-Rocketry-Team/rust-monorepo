#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use std::sync::{Arc, Barrier};

use avionics::start_avionics_thread;
use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
    window::PresentMode,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::prelude::*;
use firmware_common::driver::debugger::DebuggerTargetEvent;
use ground_test::create_ground_test;
use keyframe::animation_system;
use launch::{create_launch, ignition_handler};
use motor::{motor_ignitor, motor_system};
use rocket::{rocket_camera_tracking, rocket_chute_system};
use virt_drivers::{
    debugger::{create_debugger, DebuggerHost},
    sensors::{create_sensors, SensorSender},
    serial::{create_virtual_serial, VirtualSerial},
};
mod avionics;
mod calibration_system;
mod ground_test;
mod keyframe;
mod launch;
mod motor;
mod rocket;
mod virt_drivers;

pub const AVIONICS_X_LEN: f32 = 0.04;
pub const AVIONICS_Y_LEN: f32 = 0.02;
pub const AVIONICS_Z_LEN: f32 = 0.05;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Launch Simulator".into(),
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: Level::INFO,
                    filter: "wgpu=error,launch_simulator=trace,firmware_common=trace".to_string(),
                }),
        )
        .add_plugin(EguiPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(PanOrbitCameraPlugin)
        .add_event::<DebuggerTargetEvent>()
        .add_event::<UIEvent>()
        .add_event::<RocketEvent>()
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_startup_system(setup_virtual_avionics)
        .add_system(ui_system)
        .add_system(virtual_sensors)
        .add_system(debugger_receiver)
        .add_system(calibration_system::calibration_system)
        .add_system(animation_system)
        .add_system(create_ground_test)
        .add_system(rocket_chute_system)
        .add_system(create_launch)
        .add_system(ignition_handler)
        .add_system(motor_ignitor)
        .add_system(motor_system)
        .add_system(rocket_camera_tracking)
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
        .insert(Restitution::coefficient(0.1))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -0.1, 0.0)));

    // avionics
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Velocity {
            linvel: Vec3::ZERO,
            angvel: Vec3::ZERO,
        })
        .insert(Collider::cuboid(
            AVIONICS_X_LEN / 2.0,
            AVIONICS_Y_LEN / 2.0,
            AVIONICS_Z_LEN / 2.0,
        ))
        .insert(Restitution::coefficient(0.1))
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

fn ui_system(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut ev_ui: EventWriter<UIEvent>,
    mut ev_rocket: EventWriter<RocketEvent>,
    mut serial: Query<&mut VirtualSerial>,
    avionics_transform: Query<&Transform, With<AvionicsMarker>>,
) {
    let mut serial = serial.iter_mut().next().unwrap();
    let avionics_transform = avionics_transform.iter().next().unwrap();
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("FPS: {:.2}", 1.0 / time.delta_seconds_f64()));
        ui.label(format!(
            "Position: ({:.2}, {:.2}, {:.2})",
            avionics_transform.translation.x,
            avionics_transform.translation.y,
            avionics_transform.translation.z
        ));
        ui.heading("Preparation");
        if ui.button("Calibrate").clicked() {
            println!("calibrate");
            serial.blocking_write(&[0, 0, 0, 0, 0, 0, 0, 5]);
        }
        ui.add_space(8.0);
        ui.heading("Ground Test");
        if ui.button("Setup Rocket").clicked() {
            ev_ui.send(UIEvent::SetupGroundTest);
        }
        if ui.button("Eject Main Chute").clicked() {
            ev_rocket.send(RocketEvent::EjectMainChute);
        }
        if ui.button("Eject Drogue Chute").clicked() {
            ev_rocket.send(RocketEvent::EjectDrogueChute);
        }
        ui.add_space(8.0);
        ui.heading("Launch");
        if ui.button("Setup Rocket").clicked() {
            ev_ui.send(UIEvent::SetupLaunch);
        }
        if ui.button("Ignition").clicked() {
            ev_ui.send(UIEvent::Ignition);
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
    mut debugger_receiver: Query<&mut DebuggerHost>,
    mut ev_debugger: EventWriter<DebuggerTargetEvent>,
) {
    let mut debugger_receiver = debugger_receiver.iter_mut().next().unwrap();
    while let Some(event) = debugger_receiver.try_recv() {
        info!("debugger: {:?}", event);
        ev_debugger.send(event);
    }
}

#[derive(Component)]
pub struct AvionicsMarker;

#[derive(Component)]
struct ReadyBarrier(Option<Arc<Barrier>>);

pub enum UIEvent {
    SetupGroundTest,
    SetupLaunch,
    Ignition,
}

pub enum RocketEvent {
    EjectMainChute,
    EjectDrogueChute,
}
