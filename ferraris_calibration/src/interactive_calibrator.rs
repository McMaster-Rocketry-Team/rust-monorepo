use core::mem::replace;

use crate::{
    calibrator_inner::CalibratorInner, moving_average::SingleSumSMA, CalibrationInfo, IMUReading,
};

use defmt::info;
use either::Either;
use nalgebra::Vector3;
use Axis::*;
use Direction::*;
use Event::*;
use InteractiveCalibratorState::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Plus,
    Minus,
    Rotation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractiveCalibratorState {
    Idle,
    WaitingStill,
    State(Axis, Direction, Event),
    Success,
    Failure,
}

pub struct InteractiveCalibrator {
    state: InteractiveCalibratorState,
    inner: Either<CalibratorInner, Option<CalibrationInfo>>,
    last_reading: IMUReading,
    still_gyro_threshold_squared: f64,
    angular_acceleration_moving_avg: SingleSumSMA<Vector3<f64>, 30>, // each sample is 0.1s apart, gives a 3s window
    state_start_timestamp: f64,
}

const MINIMUM_SAMPLE_TIME: f64 = 3000.0;

impl Default for InteractiveCalibrator {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl InteractiveCalibrator {
    pub fn new(
        still_gyro_threshold: Option<f64>,
        gravity: Option<f64>,
        expected_angle: Option<f64>,
    ) -> Self {
        let still_gyro_threshold = still_gyro_threshold.unwrap_or(0.17);
        Self {
            state: Idle,
            inner: Either::Left(CalibratorInner::new(gravity, expected_angle)),
            last_reading: IMUReading::default(),
            still_gyro_threshold_squared: still_gyro_threshold * still_gyro_threshold,
            angular_acceleration_moving_avg: SingleSumSMA::new(Vector3::zeros()),
            state_start_timestamp: 0.0,
        }
    }

    pub fn get_state(&self) -> InteractiveCalibratorState {
        self.state
    }

    fn wait_still(&mut self, reading: &IMUReading) -> bool {
        let delta_time = reading.timestamp - self.last_reading.timestamp;
        if delta_time > 100.0 {
            let gyro = Vector3::from_row_slice(&reading.gyro).cast();
            let delta = gyro - Vector3::from_row_slice(&self.last_reading.gyro).cast();
            let angular_acceleration = delta / (delta_time / 1000.0);
            info!("angular_acceleration: {}", angular_acceleration.as_slice());
            self.last_reading = reading.clone();

            self.angular_acceleration_moving_avg
                .add_sample(angular_acceleration);

            if self.angular_acceleration_moving_avg.is_full()
                && self
                    .angular_acceleration_moving_avg
                    .get_average()
                    .norm_squared()
                    < self.still_gyro_threshold_squared
            {
                self.angular_acceleration_moving_avg.clear();
                return true;
            }
        }

        return false;
    }

    // Will return Some when the state changes
    pub fn process(&mut self, reading: &IMUReading) -> Option<InteractiveCalibratorState> {
        let mut new_state = None;
        let mut inner = self.inner.as_mut().unwrap_left();
        match self.state {
            Idle => {
                self.last_reading = reading.clone();
                new_state = Some(WaitingStill);
            }
            WaitingStill => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(X, Plus, Start));
                }
            }
            State(X, Plus, Start) => {
                inner.process_x_p(reading);
                if inner.x_p_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(X, Plus, End));
                }
            }
            State(X, Plus, End) => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(X, Minus, Start));
                }
            }
            State(X, Minus, Start) => {
                inner.process_x_n(reading);
                if inner.x_n_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(X, Minus, End));
                }
            }
            State(X, Minus, End) => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(Y, Plus, Start));
                }
            }
            State(Y, Plus, Start) => {
                inner.process_y_p(reading);
                if inner.y_p_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(Y, Plus, End));
                }
            }
            State(Y, Plus, End) => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(Y, Minus, Start));
                }
            }
            State(Y, Minus, Start) => {
                inner.process_y_n(reading);
                if inner.y_n_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(Y, Minus, End));
                }
            }
            State(Y, Minus, End) => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(Z, Plus, Start));
                }
            }
            State(Z, Plus, Start) => {
                inner.process_z_p(reading);
                if inner.z_p_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(Z, Plus, End));
                }
            }
            State(Z, Plus, End) => {
                if self.wait_still(&reading) {
                    self.state_start_timestamp = reading.timestamp;
                    new_state = Some(State(Z, Minus, Start));
                }
            }
            State(Z, Minus, Start) => {
                inner.process_z_n(reading);
                if inner.z_n_count > 300
                    && reading.timestamp - self.state_start_timestamp > MINIMUM_SAMPLE_TIME
                {
                    new_state = Some(State(Z, Minus, End));
                }
            }
            State(Z, Minus, End) => {
                if self.wait_still(&reading) {
                    new_state = Some(State(X, Rotation, Start));
                }
            }
            State(X, Rotation, Start) => {
                inner.process_x_rotation(reading);
                if self.wait_still(&reading) {
                    new_state = Some(State(X, Rotation, End));
                }
            }
            State(X, Rotation, End) => {
                if self.wait_still(&reading) {
                    new_state = Some(State(Y, Rotation, Start));
                }
            }
            State(Y, Rotation, Start) => {
                inner.process_y_rotation(reading);
                if self.wait_still(&reading) {
                    new_state = Some(State(Y, Rotation, End));
                }
            }
            State(Y, Rotation, End) => {
                if self.wait_still(&reading) {
                    new_state = Some(State(Z, Rotation, Start));
                }
            }
            State(Z, Rotation, Start) => {
                inner.process_z_rotation(reading);
                if self.wait_still(&reading) {
                    let old_inner = replace(&mut self.inner, Either::Right(None));
                    let cal_info = old_inner.unwrap_left().calculate();
                    new_state = if cal_info.is_some() {
                        Some(Success)
                    } else {
                        Some(Failure)
                    };
                    self.inner = Either::Right(cal_info);
                }
            }
            _ => panic!("Invalid state"),
        }

        if let Some(new_state) = new_state {
            self.state = new_state;
        }
        new_state
    }

    pub fn get_calibration_info(self) -> Option<CalibrationInfo> {
        self.inner.right().unwrap_or_default()
    }
}
