use crate::{calibrator::CalibratorInner, IMUReading};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractiveCalibratorState {
    Idle,
    BasicCalibrationStart,
    BasicCalibrationEnd,
    XPlusStart,
    XPlusEnd,
    XMinusStart,
    XMinusEnd,
    YPlusStart,
    YPlusEnd,
    YMinusStart,
    YMinusEnd,
    ZPlusStart,
    ZPlusEnd,
    ZMinusStart,
    ZMinusEnd,
    XRotationStart,
    XRotationEnd,
    YRotationStart,
    YRotationEnd,
    ZRotationStart,
    ZRotationEnd,
}

use nalgebra::Vector3;
use InteractiveCalibratorState::*;

pub struct InteractiveCalibrator {
    state: InteractiveCalibratorState,
    inner: CalibratorInner,
    basic_gyro_sum: Vector3<f64>,
    basic_gyro_count: u32,
    basic_start: f64,
    basic_gyro_offset: Vector3<f64>,
}

impl InteractiveCalibrator {
    pub fn new(gravity: Option<f64>, expected_angle: Option<f64>) -> Self {
        Self {
            state: Idle,
            inner: CalibratorInner::new(gravity, expected_angle),
            basic_gyro_sum: Vector3::zeros(),
            basic_gyro_count: 0,
            basic_start: 0.0,
            basic_gyro_offset: Vector3::zeros(),
        }
    }

    pub fn get_state(&self) -> InteractiveCalibratorState {
        self.state
    }

    // Will return Some when the state changes
    pub fn process(&mut self, reading: &IMUReading) -> Option<InteractiveCalibratorState> {
        let mut new_state = None;
        match self.state {
            Idle => {
                new_state = Some(BasicCalibrationStart);
                self.basic_start = reading.timestamp;
            }
            BasicCalibrationStart => {
                self.basic_gyro_sum += Vector3::from_row_slice(&reading.gyro).cast();
                self.basic_gyro_count += 1;

                if reading.timestamp - self.basic_start > 3000.0 && self.basic_gyro_count > 100 {
                    self.basic_gyro_offset = self.basic_gyro_sum / self.basic_gyro_count as f64;
                    new_state = Some(BasicCalibrationEnd);
                }
            }
            BasicCalibrationEnd => {

            }
            XPlusStart => todo!(),
            XPlusEnd => todo!(),
            XMinusStart => todo!(),
            XMinusEnd => todo!(),
            YPlusStart => todo!(),
            YPlusEnd => todo!(),
            YMinusStart => todo!(),
            YMinusEnd => todo!(),
            ZPlusStart => todo!(),
            ZPlusEnd => todo!(),
            ZMinusStart => todo!(),
            ZMinusEnd => todo!(),
            XRotationStart => todo!(),
            XRotationEnd => todo!(),
            YRotationStart => todo!(),
            YRotationEnd => todo!(),
            ZRotationStart => todo!(),
            ZRotationEnd => todo!(),
        }

        if let Some(new_state) = new_state {
            self.state = new_state;
        }
        new_state
    }
}
