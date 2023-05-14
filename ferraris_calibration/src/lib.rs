#![cfg_attr(not(test), no_std)]
#![feature(trivial_bounds)]

pub use calibration_info::CalibrationInfo;
pub use calibrator::{Calibrator, new_calibrator};
pub use imu_reading::IMUReading;

mod calibration_info;
mod calibrator;
mod imu_reading;
mod interactive_calibrator;
