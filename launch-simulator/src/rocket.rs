use crate::RocketEvent;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub fn rocket_chute_system(
    mut commands: Commands,
    mut ev_rocket: EventReader<RocketEvent>,
    mut nose_cone_entity: Query<(Entity, &Transform), With<RocketMarker::NoseCone>>,
    mut body_tube_entity: Query<Entity, With<RocketMarker::BodyTube>>,
    mut motor_tube_entity: Query<(Entity, &Transform), With<RocketMarker::MotorTube>>,
) {
    for event in ev_rocket.iter() {
        let (nose_cone_entity, nose_cone_transform) = nose_cone_entity.iter_mut().next().unwrap();
        let body_tube_entity = body_tube_entity.iter_mut().next().unwrap();
        let (motor_tube_entity, motor_tube_transform) =
            motor_tube_entity.iter_mut().next().unwrap();
        match event {
            RocketEvent::EjectMainChute => {
                let impulse = Vec3::new(0.0, 20.0, 0.0);
                let impulse = nose_cone_transform.rotation.inverse() * impulse;
                let rope_joint = RopeJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0))
                    .limits([0.0, 3.0]);

                commands
                    .entity(nose_cone_entity)
                    .remove::<ImpulseJoint>()
                    .insert(ExternalImpulse {
                        impulse: -impulse,
                        torque_impulse: Vec3::ZERO,
                    })
                    .insert(ImpulseJoint::new(body_tube_entity, rope_joint));

                commands.entity(body_tube_entity).insert(ExternalImpulse {
                    impulse: impulse,
                    torque_impulse: Vec3::ZERO,
                });
            }
            RocketEvent::EjectDrogueChute => {
                let impulse = Vec3::new(0.0, 30.0, 0.0);
                let impulse = motor_tube_transform.rotation.inverse() * impulse;

                let rope_joint = RopeJointBuilder::new()
                    .local_anchor1(Vec3::new(0.0, 0.0, 0.0))
                    .local_anchor2(Vec3::new(0.0, 0.0, 0.0))
                    .limits([0.0, 3.0]);

                commands
                    .entity(motor_tube_entity)
                    .remove::<ImpulseJoint>()
                    .insert(ExternalImpulse {
                        impulse: impulse,
                        torque_impulse: Vec3::ZERO,
                    })
                    .insert(ImpulseJoint::new(body_tube_entity, rope_joint));

                commands.entity(body_tube_entity).insert(ExternalImpulse {
                    impulse: -impulse,
                    torque_impulse: Vec3::ZERO,
                });
            }
        }
    }
}

pub mod RocketMarker {
    use bevy::prelude::Component;

    #[derive(Component)]
    pub struct NoseCone;

    #[derive(Component)]
    pub struct BodyTube;

    #[derive(Component)]
    pub struct MotorTube;
}
