use bevy::prelude::*;
use nalgebra::{UnitQuaternion, Vector3};

pub trait KeyFrame {
    fn lerp(&self, other: &Self, ratio: f32) -> Self;
}

impl KeyFrame for f32 {
    fn lerp(&self, other: &Self, ratio: f32) -> Self {
        self + (other - self) * ratio
    }
}

pub struct Translation3DKeyFrame {
    orientation: UnitQuaternion<f32>,
    translation: Vector3<f32>,
}

impl KeyFrame for Translation3DKeyFrame {
    fn lerp(&self, other: &Self, ratio: f32) -> Self {
        Self {
            orientation: self.orientation.slerp(&other.orientation, ratio),
            translation: self.translation.lerp(&other.translation, ratio),
        }
    }
}

impl Translation3DKeyFrame {
    pub fn new(orientation: UnitQuaternion<f32>, translation: Vector3<f32>) -> Self {
        Self {
            orientation,
            translation,
        }
    }
}

pub struct Animation<K: KeyFrame> {
    durations: Vec<f32>,
    keyframes: Vec<K>,
}

pub struct AnimationBuilder<K: KeyFrame> {
    durations: Vec<f32>,
    keyframes: Vec<K>,
}

impl<K: KeyFrame> AnimationBuilder<K> {
    pub fn new() -> Self {
        Self {
            durations: Vec::new(),
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(mut self, keyframe: K, duration: f32) -> Self {
        self.durations.push(duration);
        self.keyframes.push(keyframe);
        self
    }
    pub fn add_keyframe_absolute(mut self, keyframe: K, absolute_time: f32) -> Self {
        let last_time: f32 = self.durations.iter().sum();
        self.durations.push(absolute_time - last_time);
        self.keyframes.push(keyframe);
        self
    }

    pub fn finish(mut self, keyframe: K) -> Animation<K> {
        self.keyframes.push(keyframe);
        Animation {
            durations: self.durations,
            keyframes: self.keyframes,
        }
    }
}

#[derive(Component)]
pub struct AnimationPlayer<K: KeyFrame> {
    animation: Animation<K>,
    time: f32,
    delete_entity_on_finish: bool,
}

impl<K: KeyFrame> AnimationPlayer<K> {
    pub fn new(animation: Animation<K>, delete_entity_on_finish: bool) -> Self {
        Self {
            animation,
            time: 0.0,
            delete_entity_on_finish,
        }
    }

    pub fn update(&mut self, delta: f32) -> Option<K> {
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

        Some(keyframe.lerp(next_keyframe, ratio))
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut animated_entity: Query<(
        Entity,
        &mut AnimationPlayer<Translation3DKeyFrame>,
        &mut Transform,
    )>,
) {
    for (entity, mut animation_player, mut transform) in animated_entity.iter_mut() {
        if let Some(Translation3DKeyFrame {
            orientation,
            translation,
        }) = animation_player.update(time.delta_seconds())
        {
            transform.rotation = orientation.into();
            transform.translation = translation.into();
        } else if animation_player.delete_entity_on_finish {
            commands.entity(entity).despawn();
        }
    }
}
