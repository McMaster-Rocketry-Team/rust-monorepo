use bevy::prelude::Component;
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

    pub fn add_keyframe(mut self, keyframe: KeyFrame, duration: f32)-> Self {
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
}

impl AnimationPlayer {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            time: 0.0,
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
