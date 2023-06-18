use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{rocket::RocketMarker, AvionicsMarker, UIEvent};

pub fn create_ground_test(
    mut commands: Commands,
    mut ev_ui: EventReader<UIEvent>,
    mut avionics_entity: Query<Entity, With<AvionicsMarker>>,
) {
    for ui_event in ev_ui.iter() {
        if let UIEvent::SetupGroundTest = ui_event {
            let avionics_entity = avionics_entity.iter_mut().next().unwrap();

            let transform = Transform::from_rotation(Quat::from_axis_angle(Vec3::Z, PI / 2.0));

            let body_tube = commands
                .spawn(RigidBody::Dynamic)
                .insert(Collider::cylinder(
                    BODY_TUBE_LENGTH / 2.0,
                    ROCKET_DIAMETER / 2.0,
                ))
                .insert(Restitution::coefficient(0.1))
                .insert(Damping {
                    linear_damping: 0.0,
                    angular_damping: 2.0,
                })
                .insert(TransformBundle::from(transform.clone().with_translation(
                    Vec3::new(0.0, ROCKET_DIAMETER / 2.0 + 0.5, 1.0),
                )))
                .insert(Velocity {
                    linvel: Vec3::ZERO,
                    angvel: Vec3::ZERO,
                })
                .insert(ColliderMassProperties::Mass(BODY_TUBE_MASS))
                .insert(RocketMarker::BodyTube)
                .insert(AvionicsMarker)
                .id();

            let nose_cone_body_joint = FixedJointBuilder::new()
                .local_anchor1(Vec3::new(
                    0.0,
                    (NOSE_CONE_LENGTH + BODY_TUBE_LENGTH) / 2.0 + 0.005,
                    0.0,
                ))
                .build();
            commands
                .spawn(RigidBody::Dynamic)
                .insert(Collider::cone(
                    NOSE_CONE_LENGTH / 2.0,
                    ROCKET_DIAMETER / 2.0,
                ))
                .insert(Restitution::coefficient(0.1))
                .insert(Damping {
                    linear_damping: 0.00,
                    angular_damping: 2.0,
                })
                .insert(TransformBundle::from(transform.clone().with_translation(
                    Vec3::new(
                        -(NOSE_CONE_LENGTH + BODY_TUBE_LENGTH) / 2.0 - 0.005,
                        ROCKET_DIAMETER / 2.0 + 0.5,
                        1.0,
                    ),
                )))
                .insert(Velocity {
                    linvel: Vec3::ZERO,
                    angvel: Vec3::ZERO,
                })
                .insert(ColliderMassProperties::Mass(NOSE_CONE_MASS))
                .insert(RocketMarker::NoseCone)
                .insert(ImpulseJoint::new(body_tube, nose_cone_body_joint));

            let motor_body_joint = FixedJointBuilder::new()
                .local_anchor1(Vec3::new(
                    0.0,
                    -(MOTOR_TUBE_LENGTH + BODY_TUBE_LENGTH) / 2.0 - 0.005,
                    0.0,
                ))
                .build();
            commands
                .spawn(RigidBody::Dynamic)
                .insert(Collider::cylinder(
                    MOTOR_TUBE_LENGTH / 2.0,
                    ROCKET_DIAMETER / 2.0,
                ))
                .insert(Restitution::coefficient(0.1))
                .insert(Damping {
                    linear_damping: 0.02,
                    angular_damping: 2.0,
                })
                .insert(TransformBundle::from(transform.clone().with_translation(
                    Vec3::new(
                        (MOTOR_TUBE_LENGTH + BODY_TUBE_LENGTH) / 2.0 + 0.005,
                        ROCKET_DIAMETER / 2.0 + 0.5,
                        1.0,
                    ),
                )))
                .insert(Velocity {
                    linvel: Vec3::ZERO,
                    angvel: Vec3::ZERO,
                })
                .insert(ColliderMassProperties::Mass(MOTOR_TUBE_MASS))
                .insert(RocketMarker::MotorTube)
                .insert(ImpulseJoint::new(body_tube, motor_body_joint));

            commands.entity(avionics_entity).despawn();
        }
    }
}

const ROCKET_DIAMETER: f32 = 0.13;
const NOSE_CONE_LENGTH: f32 = 0.5;
const NOSE_CONE_MASS: f32 = 1.55;
const BODY_TUBE_LENGTH: f32 = 1.0;
const BODY_TUBE_MASS: f32 = 14.0;
const MOTOR_TUBE_LENGTH: f32 = 1.0;
const MOTOR_TUBE_MASS: f32 = 6.3 + 6.0;
