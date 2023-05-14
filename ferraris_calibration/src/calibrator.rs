use core::marker::PhantomData;

use libm::sqrt;
use nalgebra::{Matrix3, Vector3};

use crate::{calibration_info::CalibrationInfo, imu_reading::IMUReading};

pub struct XPlus {}
pub struct XMinus {}
pub struct YPlus {}
pub struct YMinus {}
pub struct ZPlus {}
pub struct ZMinus {}
pub struct XRotation {}
pub struct YRotation {}
pub struct ZRotation {}

pub(crate) struct CalibratorInner {
    gravity: f64,
    expected_angle: f64,

    x_p_count: u32,
    x_a_count: u32,
    y_p_count: u32,
    y_a_count: u32,
    z_p_count: u32,
    z_a_count: u32,

    acc_x_p_sum: Vector3<f64>,
    acc_x_a_sum: Vector3<f64>,
    acc_y_p_sum: Vector3<f64>,
    acc_y_a_sum: Vector3<f64>,
    acc_z_p_sum: Vector3<f64>,
    acc_z_a_sum: Vector3<f64>,

    gyro_x_p_sum: Vector3<f64>,
    gyro_x_a_sum: Vector3<f64>,
    gyro_y_p_sum: Vector3<f64>,
    gyro_y_a_sum: Vector3<f64>,
    gyro_z_p_sum: Vector3<f64>,
    gyro_z_a_sum: Vector3<f64>,

    x_rotation_count: u32,
    y_rotation_count: u32,
    z_rotation_count: u32,

    x_rotation_start_time_ms: f64,
    y_rotation_start_time_ms: f64,
    z_rotation_start_time_ms: f64,

    x_rotation_end_time_ms: f64,
    y_rotation_end_time_ms: f64,
    z_rotation_end_time_ms: f64,

    acc_x_rotation_sum: Vector3<f64>,
    acc_y_rotation_sum: Vector3<f64>,
    acc_z_rotation_sum: Vector3<f64>,

    gyro_x_rotation_sum: Vector3<f64>,
    gyro_y_rotation_sum: Vector3<f64>,
    gyro_z_rotation_sum: Vector3<f64>,
}

impl CalibratorInner {
    pub fn new(gravity: Option<f64>, expected_angle: Option<f64>) -> Self {
        CalibratorInner {
            gravity: gravity.unwrap_or(9.81),
            expected_angle: expected_angle.unwrap_or(-360.0),

            x_p_count: 0,
            x_a_count: 0,
            y_p_count: 0,
            y_a_count: 0,
            z_p_count: 0,
            z_a_count: 0,

            acc_x_p_sum: Vector3::zeros(),
            acc_x_a_sum: Vector3::zeros(),
            acc_y_p_sum: Vector3::zeros(),
            acc_y_a_sum: Vector3::zeros(),
            acc_z_p_sum: Vector3::zeros(),
            acc_z_a_sum: Vector3::zeros(),

            gyro_x_p_sum: Vector3::zeros(),
            gyro_x_a_sum: Vector3::zeros(),
            gyro_y_p_sum: Vector3::zeros(),
            gyro_y_a_sum: Vector3::zeros(),
            gyro_z_p_sum: Vector3::zeros(),
            gyro_z_a_sum: Vector3::zeros(),

            x_rotation_count: 0,
            y_rotation_count: 0,
            z_rotation_count: 0,

            x_rotation_start_time_ms: 0.0,
            y_rotation_start_time_ms: 0.0,
            z_rotation_start_time_ms: 0.0,

            x_rotation_end_time_ms: 0.0,
            y_rotation_end_time_ms: 0.0,
            z_rotation_end_time_ms: 0.0,

            acc_x_rotation_sum: Vector3::zeros(),
            acc_y_rotation_sum: Vector3::zeros(),
            acc_z_rotation_sum: Vector3::zeros(),

            gyro_x_rotation_sum: Vector3::zeros(),
            gyro_y_rotation_sum: Vector3::zeros(),
            gyro_z_rotation_sum: Vector3::zeros(),
        }
    }
}

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

impl Calibrator<XPlus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.x_p_count += 1;
        self.inner.acc_x_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_x_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<XMinus> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<XMinus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.x_a_count += 1;
        self.inner.acc_x_a_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_x_a_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<YPlus> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<YPlus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.y_p_count += 1;
        self.inner.acc_y_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_y_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<YMinus> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<YMinus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.y_a_count += 1;
        self.inner.acc_y_a_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_y_a_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<ZPlus> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<ZPlus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.z_p_count += 1;
        self.inner.acc_z_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_z_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<ZMinus> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<ZMinus> {
    pub fn process(&mut self, reading: &IMUReading) {
        self.inner.z_a_count += 1;
        self.inner.acc_z_a_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_z_a_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn next(self) -> Calibrator<XRotation> {
        Calibrator {
            phantom: PhantomData,
            inner: self.inner,
        }
    }
}

impl Calibrator<XRotation> {
    pub fn process(&mut self, reading: &IMUReading) {
        if self.inner.x_rotation_count == 0 {
            self.inner.x_rotation_start_time_ms = reading.timestamp;
        }
        self.inner.x_rotation_end_time_ms = reading.timestamp;
        self.inner.x_rotation_count += 1;
        self.inner.acc_x_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_x_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
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
        if self.inner.y_rotation_count == 0 {
            self.inner.y_rotation_start_time_ms = reading.timestamp;
        }
        self.inner.y_rotation_end_time_ms = reading.timestamp;
        self.inner.y_rotation_count += 1;
        self.inner.acc_y_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_y_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
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
        if self.inner.z_rotation_count == 0 {
            self.inner.z_rotation_start_time_ms = reading.timestamp;
        }
        self.inner.z_rotation_end_time_ms = reading.timestamp;
        self.inner.z_rotation_count += 1;
        self.inner.acc_z_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.inner.gyro_z_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    #[allow(non_snake_case)]
    pub fn calculate(mut self) -> CalibrationInfo {
        let mut inner = self.inner;
        // Compute Acceleration Matrix

        // Calculate means from all static phases and stack them into 3x3 matrices
        // Note: Each measurement should be a column
        let U_a_p = Matrix3::from_columns(&[
            inner.acc_x_p_sum / inner.x_p_count as f64,
            inner.acc_y_p_sum / inner.y_p_count as f64,
            inner.acc_z_p_sum / inner.z_p_count as f64,
        ]);

        let U_a_n = Matrix3::from_columns(&[
            inner.acc_x_a_sum / inner.x_a_count as f64,
            inner.acc_y_a_sum / inner.y_a_count as f64,
            inner.acc_z_a_sum / inner.z_a_count as f64,
        ]);

        // Eq. 19
        let U_a_s = U_a_p + U_a_n;

        // Bias Matrix
        // Eq. 20
        let B_a = U_a_s / 2.0;

        // Bias Vector
        let b_a = B_a.diagonal();

        // Compute Scaling and Rotation
        // No need for bias correction, since it cancels out!
        // Eq. 21
        let U_a_d = U_a_p - U_a_n;

        // Calculate Scaling matrix
        // Eq. 23
        let k_a_sq =
            1.0 / (4.0 * inner.gravity * inner.gravity) * (U_a_d * U_a_d.transpose()).diagonal();
        let K_a = Matrix3::from_diagonal(&k_a_sq.map(sqrt));

        // Calculate Rotation matrix
        // Eq. 22
        let R_a = K_a.try_inverse().unwrap() * (U_a_d / (2.0 * inner.gravity));

        // Calculate Gyroscope Matrix

        // Gyro Bias from the static phases of the acc calibration
        // One static phase would be sufficient, but why not use all of them if you have them.
        // Note that this calibration ignores any influences due to the earth rotation.
        let b_g = (inner.gyro_x_p_sum
            + inner.gyro_y_p_sum
            + inner.gyro_z_p_sum
            + inner.gyro_x_a_sum
            + inner.gyro_y_a_sum
            + inner.gyro_z_a_sum)
            / (inner.x_p_count
                + inner.y_p_count
                + inner.z_p_count
                + inner.x_a_count
                + inner.y_a_count
                + inner.z_a_count) as f64;

        // Acceleration sensitivity
        let U_g_p = Matrix3::from_columns(&[
            inner.gyro_x_p_sum / inner.x_p_count as f64,
            inner.gyro_y_p_sum / inner.y_p_count as f64,
            inner.gyro_z_p_sum / inner.z_p_count as f64,
        ]);
        let U_g_a = Matrix3::from_columns(&[
            inner.gyro_x_a_sum / inner.x_a_count as f64,
            inner.gyro_y_a_sum / inner.y_a_count as f64,
            inner.gyro_z_a_sum / inner.z_a_count as f64,
        ]);

        // Eq. 9
        let K_ga = (U_g_p - U_g_a) / (2.0 * inner.gravity);

        // Gyroscope Scaling and Rotation

        // First apply partial calibration to remove offset and acceleration influence
        let acc_mat = R_a.try_inverse().unwrap() * K_a.try_inverse().unwrap();
        let apply_calibration =
            |acc_sum: &mut Vector3<f64>, gyro_sum: &mut Vector3<f64>, count: u32| {
                *acc_sum = acc_mat * (*acc_sum - count as f64 * b_a);
                *gyro_sum = *gyro_sum - K_ga * *acc_sum - count as f64 * b_g;
            };
        apply_calibration(
            &mut inner.acc_x_rotation_sum,
            &mut inner.gyro_x_rotation_sum,
            inner.x_rotation_count,
        );
        apply_calibration(
            &mut inner.acc_y_rotation_sum,
            &mut inner.gyro_y_rotation_sum,
            inner.y_rotation_count,
        );
        apply_calibration(
            &mut inner.acc_z_rotation_sum,
            &mut inner.gyro_z_rotation_sum,
            inner.z_rotation_count,
        );

        // Integrate gyro readings
        // Eq. 13/14
        let W_s = Matrix3::from_columns(&[
            inner.gyro_x_rotation_sum
                / (inner.x_rotation_count as f64
                    / ((inner.x_rotation_end_time_ms - inner.x_rotation_start_time_ms) as f64
                        / 1000.0)),
            inner.gyro_y_rotation_sum
                / (inner.y_rotation_count as f64
                    / ((inner.y_rotation_end_time_ms - inner.y_rotation_start_time_ms) as f64
                        / 1000.0)),
            inner.gyro_z_rotation_sum
                / (inner.z_rotation_count as f64
                    / ((inner.z_rotation_end_time_ms - inner.z_rotation_start_time_ms) as f64
                        / 1000.0)),
        ]);

        // Eq. 15
        let expected_angles = inner.expected_angle * Matrix3::identity();
        let multiplied = W_s * expected_angles.try_inverse().unwrap();

        // Eq. 12
        let k_g_sq = (multiplied * multiplied.transpose()).diagonal();
        let K_g = Matrix3::from_diagonal(&k_g_sq.map(sqrt));

        let R_g = K_g.try_inverse().unwrap() * multiplied;

        CalibrationInfo::from_raw(
            K_a.cast(),
            R_a.cast(),
            b_a.cast(),
            K_g.cast(),
            R_g.cast(),
            K_ga.cast(),
            b_g.cast(),
        )
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
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
        for imu_reading in &read_file("./calibration_data/x_minus.csv") {
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
        for imu_reading in &read_file("./calibration_data/z_plus.csv") {
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

        let cal_info = calibrator.calculate();

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
    }
}
