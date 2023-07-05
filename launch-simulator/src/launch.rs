use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use nalgebra::{UnitQuaternion, Vector3};

use crate::{
    keyframe::{AnimationBuilder, AnimationPlayer, Translation3DKeyFrame},
    rocket::RocketMarker,
    UIEvent,
};

pub fn create_launch(
    mut commands: Commands,
    mut ev_ui: EventReader<UIEvent>,
    body_tube_entity: Query<(Entity, &Transform), With<RocketMarker::BodyTube>>,
) {
    for ui_event in ev_ui.iter() {
        if let UIEvent::SetupLaunch = ui_event {
            let (body_tube, body_tube_transform) = body_tube_entity.iter().next().unwrap();

            let joint = FixedJointBuilder::new()
                .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                .local_anchor2(Vec3::new(0.0, 0.0, 0.0));

            let animation = AnimationBuilder::new()
                .add_keyframe(
                    Translation3DKeyFrame::new(
                        body_tube_transform.rotation.into(),
                        body_tube_transform.translation.into(),
                    ),
                    1.0,
                )
                .add_keyframe(
                    Translation3DKeyFrame::new(
                        body_tube_transform.rotation.into(),
                        Vector3::new(0.0, 2.0, 0.0),
                    ),
                    1.0,
                )
                .finish(Translation3DKeyFrame::new(
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0),
                    Vector3::new(0.0, 2.0, 0.0),
                ));

            let player = AnimationPlayer::new(animation, false);

            // launch mount
            commands
                .spawn(RigidBody::KinematicPositionBased)
                .insert(LaunchMountMarker)
                .insert(TransformBundle::from_transform(body_tube_transform.clone()))
                .insert(ImpulseJoint::new(body_tube, joint))
                .insert(player);
        }
    }
}

pub fn set_launch_angle(
    mut commands: Commands,
    mut ev_ui: EventReader<UIEvent>,
    launch_mount_entity: Query<(Entity, &Transform), With<LaunchMountMarker>>,
) {
    for ui_event in ev_ui.iter() {
        if let UIEvent::SetLaunchAngle(angle) = ui_event {
            let (launch_mount, launch_mount_transform) = launch_mount_entity.iter().next().unwrap();

            let animation = AnimationBuilder::new()
                .add_keyframe(
                    Translation3DKeyFrame::new(
                        launch_mount_transform.rotation.into(),
                        launch_mount_transform.translation.into(),
                    ),
                    1.0,
                )
                .finish(Translation3DKeyFrame::new(
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), *angle),
                    Vector3::new(0.0, 2.0, 0.0),
                ));

            let player = AnimationPlayer::new(animation, false);

            commands.entity(launch_mount).insert(player);
        }
    }
}

pub fn ignition_handler(
    mut commands: Commands,
    mut ev_ui: EventReader<UIEvent>,
    launch_mount_entity: Query<Entity, With<LaunchMountMarker>>,
) {
    for ui_event in ev_ui.iter() {
        if let UIEvent::Ignition = ui_event {
            let launch_mount_entity = launch_mount_entity.iter().next().unwrap();
            commands.entity(launch_mount_entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct LaunchMountMarker;
