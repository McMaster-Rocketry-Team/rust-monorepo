#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use std::{
    f32::consts::PI,
    sync::{Arc, Barrier},
};

use avionics::start_avionics_thread;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::prelude::*;
use firmware_common::driver::debugger::InteractiveCalibratorState as CalibratorState;
use firmware_common::driver::debugger::{Axis, DebuggerEvent, Direction, Event};
use keyframe::AnimationPlayer;
use nalgebra::{UnitQuaternion, Vector3};
use virt_drivers::{
    debugger::{create_debugger, DebuggerReceiver},
    sensors::{create_sensors, SensorSender},
    serial::{create_virtual_serial, VirtualSerial},
};

use crate::keyframe::{AnimationBuilder, KeyFrame};

mod avionics;
mod keyframe;
mod virt_drivers;

const AVIONICS_X_LEN: f32 = 0.04;
const AVIONICS_Y_LEN: f32 = 0.02;
const AVIONICS_Z_LEN: f32 = 0.05;

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
        .add_system(ui_system)
        .add_system(virtual_sensors)
        .add_system(debugger_receiver)
        .add_system(calibration_system)
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
    mut avionics_holder: Query<
        (Entity, &mut AnimationPlayer, &mut Transform),
        With<AvionicsHolderMarker>,
    >,
) {
    if let Some((entity, mut animation_player, mut transform)) = avionics_holder.iter_mut().next() {
        if let Some((orientation, translation)) = animation_player.update(time.delta_seconds()) {
            transform.rotation = orientation.into();
            transform.translation = translation.into();
        } else {
            commands.entity(entity).despawn();
        }
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
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();

                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::X,
                Direction::Plus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::X,
                Direction::Minus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Y,
                Direction::Plus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Y,
                Direction::Minus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Z,
                Direction::Plus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Z,
                Direction::Minus,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::X,
                Direction::Rotation,
                Event::Start,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset =
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::X,
                Direction::Rotation,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset =
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Y,
                Direction::Rotation,
                Event::Start,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Y,
                Direction::Rotation,
                Event::End,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::State(
                Axis::Z,
                Direction::Rotation,
                Event::Start,
            )) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset =
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);
                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerEvent::Calibrating(CalibratorState::Success | CalibratorState::Failure) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();
                let rotation_offset =
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI);

                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        KeyFrame::new(rotation_offset, transform.translation.into()),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(rotation_offset, Vector3::new(0.0, 0.1, 0.0)),
                        1.0,
                    )
                    .add_keyframe(
                        KeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(KeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
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
