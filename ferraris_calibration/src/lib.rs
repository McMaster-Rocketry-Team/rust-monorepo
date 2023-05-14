#![cfg_attr(not(test), no_std)]
#![feature(trivial_bounds)]

pub use calibration_info::CalibrationInfo;
pub use calibrator::{new_calibrator, Calibrator};
pub use imu_reading::IMUReading;
pub use interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibrator, InteractiveCalibratorState,
};

mod calibration_info;
mod calibrator;
pub mod calibrator_inner;
mod imu_reading;
mod interactive_calibrator;
mod moving_average;
