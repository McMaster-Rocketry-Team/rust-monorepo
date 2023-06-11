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
use firmware_common::driver::debugger::InteractiveCalibratorState as CalibratorState;
use nalgebra::UnitQuaternion;
use virt_drivers::{
    debugger::{create_debugger, DebuggerReceiver},
    sensors::{create_sensors, SensorSender},
    serial::{create_virtual_serial, VirtualSerial},
};

mod avionics;
mod virt_drivers;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Launch Simulator".into(),
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
        .add_event::<DebuggerEvent>()
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_startup_system(setup_virtual_avionics)
        .add_startup_system(setup_interpolation)
        .add_system(ui_system)
        .add_system(virtual_sensors)
        .add_system(debugger_receiver)
        .add_system(calibration_system)
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

fn setup_interpolation(mut commands: Commands) {
    commands.spawn(OrientationInterpolation::default());
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
        Query<(&Velocity, &Transform, With<AvionicsMarker>)>,
        Query<&mut ReadyBarrier>,
        Query<&mut SensorSender>,
    ),
) {
    let (vel, pos, _) = state.iter().next().unwrap();

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

fn calibration_system(
    mut commands: Commands,
    mut ev_debugger: EventReader<DebuggerEvent>,
    avionics_transform: Query<&Transform, With<AvionicsMarker>>,
    avionics_entity: Query<Entity, With<AvionicsMarker>>,
) {
    let avionics_transform = avionics_transform.iter().next().unwrap();
    let avionics_entity = avionics_entity.iter().next().unwrap();
    for ev in ev_debugger.iter() {
        match ev {
            DebuggerEvent::Calibrating(CalibratorState::WaitingStill) => {
                println!("creating joint");
                let joint = PrismaticJointBuilder::new(Vec3::X)
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint));
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct AvionicsMarker;

#[derive(Component)]
struct AvionicsHolderMarker;

#[derive(Component)]
struct ReadyBarrier(Option<Arc<Barrier>>);

#[derive(Component, Default)]
struct OrientationInterpolation {
    start: UnitQuaternion<f32>,
    end: UnitQuaternion<f32>,
    start_time: f64,
    end_time: f64,
    finished: bool,
}
