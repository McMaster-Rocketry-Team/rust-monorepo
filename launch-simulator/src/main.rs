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
use virt_drivers::{
    debugger::create_debugger,
    sensors::{create_sensors, SensorSender},
    serial::{create_virtual_serial, VirtualSerial},
};

mod avionics;
mod virt_drivers;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // title: "I am a window!".into(),
                // resolution: (500., 300.).into(),
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
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_startup_system(setup_virtual_avionics)
        .add_system(ui_system)
        .add_system(virtual_sensors)
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
        .insert(Collider::cuboid(0.04 / 2.0, 0.02 / 2.0, 0.05 / 2.0))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.1, 0.0)));
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
    });
}

fn virtual_sensors(
    (state, mut ready_barrier, mut sensor_tx): (
        Query<(&Velocity, &Transform)>,
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

#[derive(Component)]
struct ReadyBarrier(Option<Arc<Barrier>>);
