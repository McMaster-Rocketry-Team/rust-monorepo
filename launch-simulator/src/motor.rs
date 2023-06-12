use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    keyframe::{AnimationBuilder, AnimationPlayer},
    rocket::RocketMarker,
    UIEvent,
};

type ThrustPlayer = AnimationPlayer<f32>;

pub fn motor_ignitor(
    mut commands: Commands,
    mut ev_ui: EventReader<UIEvent>,
    motor_tube_entity: Query<Entity, With<RocketMarker::MotorTube>>,
) {
    for ui_event in ev_ui.iter() {
        if let UIEvent::Ignition = ui_event {
            let motor_tube_entity = motor_tube_entity.iter().next().unwrap();
            let thrust_curve = AnimationBuilder::new()
                .add_keyframe_absolute(0.0, 0.2)
                .add_keyframe_absolute(2250.0, 1.3)
                .add_keyframe_absolute(2600.0, 2.9)
                .add_keyframe_absolute(2200.0, 3.5)
                .finish(0.0);

            let thrust_player: ThrustPlayer = AnimationPlayer::new(thrust_curve, false);
            commands.entity(motor_tube_entity).insert(thrust_player);
        }
    }
}

pub fn motor_system(
    mut commands: Commands,
    time: Res<Time>,
    mut motor_tube_entity: Query<
        (Entity, &mut ThrustPlayer, &Transform),
        With<RocketMarker::MotorTube>,
    >,
) {
    for (entity, mut thrust_player, transform) in motor_tube_entity.iter_mut() {
        if let Some(thrust) = thrust_player.update(time.delta_seconds()) {
            let thrust = Vec3::new(0.0, thrust, 0.0);
            let thrust = transform.rotation * thrust;
            commands.entity(entity).insert(ExternalForce {
                force: thrust,
                torque: Vec3::new(0.0, 0.0, 0.0),
            });
        } else {
            commands.entity(entity).remove::<ThrustPlayer>();
        }
    }
}
