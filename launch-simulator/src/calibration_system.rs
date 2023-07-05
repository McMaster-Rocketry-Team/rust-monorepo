use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use firmware_common::driver::debugger::InteractiveCalibratorState as CalibratorState;
use firmware_common::driver::debugger::{Axis, DebuggerTargetEvent, Direction, Event};
use nalgebra::{UnitQuaternion, Vector3};

use crate::{
    keyframe::{AnimationBuilder, AnimationPlayer, Translation3DKeyFrame},
    AvionicsMarker, AVIONICS_X_LEN, AVIONICS_Y_LEN, AVIONICS_Z_LEN,
};

pub fn calibration_system(
    mut commands: Commands,
    mut ev_debugger: EventReader<DebuggerTargetEvent>,
    avionics_transform: Query<&Transform, With<AvionicsMarker>>,
    avionics_entity: Query<Entity, With<AvionicsMarker>>,
) {
    let avionics_transform = avionics_transform.iter().next().unwrap();
    let avionics_entity = avionics_entity.iter().next().unwrap();
    for ev in ev_debugger.iter() {
        match ev {
            DebuggerTargetEvent::Calibrating(CalibratorState::WaitingStill) => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

                let transform = avionics_transform.clone();

                let animation = AnimationBuilder::new()
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_X_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                                * rotation_offset,
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)
                            * rotation_offset,
                        Vector3::new(0.0, AVIONICS_Z_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(CalibratorState::State(
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
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0 * 3.0)
                                * rotation_offset,
                            transform.translation.into(),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0) * rotation_offset,
                        transform.translation.into(),
                    ));

                let player = AnimationPlayer::new(animation, true);

                // avionics mount
                commands
                    .spawn(RigidBody::KinematicPositionBased)
                    .insert(AvionicsHolderMarker)
                    .insert(TransformBundle::from(transform))
                    .insert(ImpulseJoint::new(avionics_entity, joint))
                    .insert(player);
            }
            DebuggerTargetEvent::Calibrating(
                CalibratorState::Success | CalibratorState::Failure,
            ) => {
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
                        Translation3DKeyFrame::new(rotation_offset, transform.translation.into()),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(rotation_offset, Vector3::new(0.0, 0.1, 0.0)),
                        1.0,
                    )
                    .add_keyframe(
                        Translation3DKeyFrame::new(
                            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                            Vector3::new(0.0, 0.1, 0.0),
                        ),
                        1.0,
                    )
                    .finish(Translation3DKeyFrame::new(
                        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                        Vector3::new(0.0, AVIONICS_Y_LEN / 2.0, 0.0),
                    ));

                let player = AnimationPlayer::new(animation, true);

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
pub struct AvionicsHolderMarker;
