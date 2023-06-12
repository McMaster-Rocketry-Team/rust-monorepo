use bevy::prelude::*;
use nalgebra::{UnitQuaternion, Vector3};

pub struct KeyFrame {
    orientation: UnitQuaternion<f32>,
    translation: Vector3<f32>,
}

impl KeyFrame {
    pub fn new(orientation: UnitQuaternion<f32>, translation: Vector3<f32>) -> Self {
        Self {
            orientation,
            translation,
        }
    }
}

pub struct Animation {
    durations: Vec<f32>,
    keyframes: Vec<KeyFrame>,
}

pub struct AnimationBuilder {
    durations: Vec<f32>,
    keyframes: Vec<KeyFrame>,
}

impl AnimationBuilder {
    pub fn new() -> Self {
        Self {
            durations: Vec::new(),
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(mut self, keyframe: KeyFrame, duration: f32) -> Self {
        self.durations.push(duration);
        self.keyframes.push(keyframe);
        self
    }

    pub fn finish(mut self, keyframe: KeyFrame) -> Animation {
        self.keyframes.push(keyframe);
        Animation {
            durations: self.durations,
            keyframes: self.keyframes,
        }
    }
}

#[derive(Component)]
pub struct AnimationPlayer {
    animation: Animation,
    time: f32,
    delete_entity_on_finish: bool,
}

impl AnimationPlayer {
    pub fn new(animation: Animation, delete_entity_on_finish: bool) -> Self {
        Self {
            animation,
            time: 0.0,
            delete_entity_on_finish,
        }
    }

    pub fn update(&mut self, delta: f32) -> Option<(UnitQuaternion<f32>, Vector3<f32>)> {
        self.time += delta;
        let mut time = self.time;
        let mut keyframe_index = 0;
        while time > self.animation.durations[keyframe_index] {
            time -= self.animation.durations[keyframe_index];
            keyframe_index += 1;
            if keyframe_index >= self.animation.durations.len() {
                return None;
            }
        }
        let keyframe = &self.animation.keyframes[keyframe_index];
        let next_keyframe = &self.animation.keyframes[keyframe_index + 1];
        let ratio = time / self.animation.durations[keyframe_index];
        let orientation = keyframe
            .orientation
            .slerp(&next_keyframe.orientation, ratio);
        let translation = keyframe.translation.lerp(&next_keyframe.translation, ratio);
        Some((orientation, translation))
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut animated_entity: Query<(Entity, &mut AnimationPlayer, &mut Transform)>,
) {
    for (entity, mut animation_player, mut transform) in animated_entity.iter_mut() {
        if let Some((orientation, translation)) = animation_player.update(time.delta_seconds()) {
            transform.rotation = orientation.into();
            transform.translation = translation.into();
        } else if animation_player.delete_entity_on_finish {
            commands.entity(entity).despawn();
        }
    }
}
