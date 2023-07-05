use core::marker::PhantomData;

use crate::{
    calibration_info::CalibrationInfo, calibrator_inner::CalibratorInner, imu_reading::IMUReading,
};

pub struct Calibrator<S> {
    phantom: PhantomData<S>,
    inner: CalibratorInner,
}

pub fn new_calibrator(gravity: Option<f64>, expected_angle: Option<f64>) -> Calibrator<XPlus> {
    Calibrator {
        phantom: PhantomData,
        inner: CalibratorInner::new(gravity, expected_angle),
    }
}

macro_rules! impl_type_state {
    ($state: ident, $method: ident, $next_state: ident) => {
        pub struct $state {}

        impl Calibrator<$state> {
            pub fn process(&mut self, reading: &IMUReading) {
                self.inner.$method(reading);
            }

            pub fn next(self) -> Calibrator<$next_state> {
                Calibrator {
                    phantom: PhantomData,
                    inner: self.inner,
                }
            }
        }
    };
}

impl_type_state!(XPlus, process_x_p, XPlusVariance);
impl_type_state!(XPlusVariance, process_x_p_variance, XMinus);
impl_type_state!(XMinus, process_x_n, XMinusVariance);
impl_type_state!(XMinusVariance, process_x_n_variance, YPlus);
impl_type_state!(YPlus, process_y_p, YPlusVariance);
impl_type_state!(YPlusVariance, process_y_p_variance, YMinus);
impl_type_state!(YMinus, process_y_n, YMinusVariance);
impl_type_state!(YMinusVariance, process_y_n_variance, ZPlus);
impl_type_state!(ZPlus, process_z_p, ZPlusVariance);
impl_type_state!(ZPlusVariance, process_z_p_variance, ZMinus);
impl_type_state!(ZMinus, process_z_n, ZMinusVariance);
impl_type_state!(ZMinusVariance, process_z_n_variance, XRotation);

pub struct XRotation {}
pub struct YRotation {}
pub struct ZRotation {}

impl Calibrator<XRotation> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.process_x_rotation(reading);
    }

    pub fn next(self) -> Calibrator<YRotation> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<YRotation> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.process_y_rotation(reading);
    }

    pub fn next(self) -> Calibrator<ZRotation> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<ZRotation> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.process_z_rotation(reading);
    }

    pub fn calculate(self) -> Option<CalibrationInfo> {
        self.inner.calculate()
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use nalgebra::{Matrix3, Vector3};
    extern crate alloc;
    use std::path::Path;
    use std::vec::Vec;

    use super::*;

    fn read_file<P: AsRef<Path>>(path: P) -> Vec<IMUReading> {
        let mut readings: Vec<IMUReading> = Vec::new();

        for line in csv::Reader::from_path(path).unwrap().records() {
            let record = line.unwrap();
            readings.push(IMUReading {
                timestamp: record[0].parse::<f64>().unwrap(),
                acc: [
                    record[1].parse::<f32>().unwrap(),
                    record[2].parse::<f32>().unwrap(),
                    record[3].parse::<f32>().unwrap(),
                ],
                gyro: [
                    record[4].parse::<f32>().unwrap(),
                    record[5].parse::<f32>().unwrap(),
                    record[6].parse::<f32>().unwrap(),
                ],
            });
        }

        readings
    }

    #[test]
    fn calculate_calibration() {
        let mut calibrator = new_calibrator(None, None);
        for imu_reading in &read_file("./calibration_data/x_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/x_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/x_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/x_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/y_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/y_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/y_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/y_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/z_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/z_plus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/z_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/z_minus.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();

        for imu_reading in &read_file("./calibration_data/gyro_x.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/gyro_y.csv") {
            calibrator.process(imu_reading);
        }
        let mut calibrator = calibrator.next();
        for imu_reading in &read_file("./calibration_data/gyro_z.csv") {
            calibrator.process(imu_reading);
        }

        let cal_info = calibrator.calculate().unwrap();

        assert_abs_diff_eq!(
            cal_info.b_a,
            Vector3::new(
                0.004518432617187498,
                -0.003014831542968749,
                0.008571166992187504
            ),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            cal_info.K_ga,
            Matrix3::new(
                -0.00048633969076576476,
                0.0018792165651189395,
                3.1125740209010755e-05,
                -0.00029958524951171435,
                -0.0019142330228540765,
                9.337722062703084e-05,
                -0.0015173798351891894,
                -0.002610671460030661,
                -0.0011749966928901107
            ),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            cal_info.b_g,
            Vector3::new(
                -0.6039185750636132,
                0.16660305343511447,
                -0.36804071246819337
            ),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            cal_info.acc_mat,
            Matrix3::new(
                -77.75729696,
                -3.07430402,
                -3.52181471,
                -3.9574844,
                78.12866309,
                4.96302269,
                0.47867244,
                4.65889686,
                -76.99118666
            ),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            cal_info.gyro_mat,
            Matrix3::new(
                -8.11168804,
                -0.38991883,
                -0.41350072,
                -0.37157825,
                7.95932724,
                0.72453155,
                -0.16215739,
                0.82079149,
                -7.9426161,
            ),
            epsilon = 0.05
        );
        assert_abs_diff_eq!(
            cal_info.acc_variance,
            Vector3::new(0.00069620233, 0.00049620867, 0.0009553565),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            cal_info.gyro_variance,
            Vector3::new(0.17576145, 0.07144321, 0.12894574),
            epsilon = 0.01
        );
    }
}
