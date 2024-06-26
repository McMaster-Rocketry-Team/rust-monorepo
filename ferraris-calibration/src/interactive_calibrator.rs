use core::mem::replace;

use crate::{
    calibrator_inner::CalibratorInner, moving_average::SingleSumSMA, CalibrationInfo,
    IMUReadingTrait,
};

use defmt::info;
use either::Either;
use nalgebra::Vector3;
use paste::paste;
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
    Variance,
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

pub struct InteractiveCalibrator<T: IMUReadingTrait> {
    state: InteractiveCalibratorState,
    inner: Either<CalibratorInner, Option<CalibrationInfo>>,
    last_reading: Option<T>,
    still_gyro_threshold_squared: f64,
    angular_acceleration_moving_avg: SingleSumSMA<Vector3<f64>, 50>, // each sample is 0.1s apart, gives a 5s window
    state_start_timestamp: Option<f64>,
}

const MINIMUM_SAMPLE_TIME: f64 = 3000.0;

impl<T: IMUReadingTrait> Default for InteractiveCalibrator<T> {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

macro_rules! arm_process {
    ($self: ident, $method: ident, $reading: expr, $inner: ident, $new_state: ident, $next: expr) => {{
        paste! {
            if $self.state_start_timestamp.is_none() {
                $self.state_start_timestamp = Some($reading.timestamp());
            }

            $inner.[<process_ $method>]($reading);
            if $inner.[<$method _count>] > 300
                && $reading.timestamp() - $self.state_start_timestamp.unwrap() > MINIMUM_SAMPLE_TIME
            {
                $self.state_start_timestamp = None;
                $new_state = Some($next);
            }
        }
    }};
}

impl<T: IMUReadingTrait> InteractiveCalibrator<T> {
    pub fn new(
        still_gyro_threshold: Option<f64>,
        gravity: Option<f64>,
        expected_angle: Option<f64>,
    ) -> Self {
        let still_gyro_threshold = still_gyro_threshold.unwrap_or(0.17);
        Self {
            state: Idle,
            inner: Either::Left(CalibratorInner::new(gravity, expected_angle)),
            last_reading: None,
            still_gyro_threshold_squared: still_gyro_threshold * still_gyro_threshold,
            angular_acceleration_moving_avg: SingleSumSMA::new(Vector3::zeros()),
            state_start_timestamp: None,
        }
    }

    pub fn get_state(&self) -> InteractiveCalibratorState {
        self.state
    }

    fn wait_still(&mut self, reading: &T) -> bool {
        if let Some(last_reading) = &self.last_reading {
            let delta_time = reading.timestamp() - last_reading.timestamp();
            if delta_time > 100.0 {
                let gyro = Vector3::from_row_slice(&reading.gyro()).cast();
                let delta = gyro - Vector3::from_row_slice(&last_reading.gyro()).cast();
                let angular_acceleration = delta / (delta_time / 1000.0);
                info!("angular_acceleration: {}", angular_acceleration.as_slice());
                self.last_reading = Some(reading.clone());

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
        } else {
            self.last_reading = Some(reading.clone());
        }

        return false;
    }

    // Will return Some when the state changes
    pub fn process(&mut self, reading: &T) -> Option<InteractiveCalibratorState> {
        let mut new_state = None;
        let inner = self.inner.as_mut().unwrap_left();
        match self.state {
            Idle => {
                self.last_reading = Some(reading.clone());
                new_state = Some(WaitingStill);
            }
            WaitingStill => {
                if self.wait_still(reading) {
                    new_state = Some(State(X, Plus, Start));
                }
            }
            State(X, Plus, Start) => arm_process!(
                self,
                x_p,
                reading,
                inner,
                new_state,
                State(X, Plus, Variance)
            ),
            State(X, Plus, Variance) => arm_process!(
                self,
                x_p_variance,
                reading,
                inner,
                new_state,
                State(X, Plus, End)
            ),
            State(X, Plus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(X, Minus, Start));
                }
            }
            State(X, Minus, Start) => arm_process!(
                self,
                x_n,
                reading,
                inner,
                new_state,
                State(X, Minus, Variance)
            ),
            State(X, Minus, Variance) => arm_process!(
                self,
                x_n_variance,
                reading,
                inner,
                new_state,
                State(X, Minus, End)
            ),
            State(X, Minus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Y, Plus, Start));
                }
            }
            State(Y, Plus, Start) => arm_process!(
                self,
                y_p,
                reading,
                inner,
                new_state,
                State(Y, Plus, Variance)
            ),
            State(Y, Plus, Variance) => arm_process!(
                self,
                y_p_variance,
                reading,
                inner,
                new_state,
                State(Y, Plus, End)
            ),
            State(Y, Plus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Y, Minus, Start));
                }
            }
            State(Y, Minus, Start) => arm_process!(
                self,
                y_n,
                reading,
                inner,
                new_state,
                State(Y, Minus, Variance)
            ),
            State(Y, Minus, Variance) => arm_process!(
                self,
                y_n_variance,
                reading,
                inner,
                new_state,
                State(Y, Minus, End)
            ),
            State(Y, Minus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Z, Plus, Start));
                }
            }
            State(Z, Plus, Start) => arm_process!(
                self,
                z_p,
                reading,
                inner,
                new_state,
                State(Z, Plus, Variance)
            ),
            State(Z, Plus, Variance) => arm_process!(
                self,
                z_p_variance,
                reading,
                inner,
                new_state,
                State(Z, Plus, End)
            ),
            State(Z, Plus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Z, Minus, Start));
                }
            }
            State(Z, Minus, Start) => arm_process!(
                self,
                z_n,
                reading,
                inner,
                new_state,
                State(Z, Minus, Variance)
            ),
            State(Z, Minus, Variance) => arm_process!(
                self,
                z_n_variance,
                reading,
                inner,
                new_state,
                State(Z, Minus, End)
            ),
            State(Z, Minus, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(X, Rotation, Start));
                }
            }
            State(X, Rotation, Start) => {
                inner.process_x_rotation(reading);
                if self.wait_still(reading) {
                    new_state = Some(State(X, Rotation, End));
                }
            }
            State(X, Rotation, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Y, Rotation, Start));
                }
            }
            State(Y, Rotation, Start) => {
                inner.process_y_rotation(reading);
                if self.wait_still(reading) {
                    new_state = Some(State(Y, Rotation, End));
                }
            }
            State(Y, Rotation, End) => {
                if self.wait_still(reading) {
                    new_state = Some(State(Z, Rotation, Start));
                }
            }
            State(Z, Rotation, Start) => {
                inner.process_z_rotation(reading);
                if self.wait_still(reading) {
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
