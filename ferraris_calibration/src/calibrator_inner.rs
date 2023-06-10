use libm::sqrt;
use nalgebra::{Matrix3, Vector3};

use crate::{CalibrationInfo, IMUReading};

pub struct CalibratorInner {
    pub gravity: f64,
    pub expected_angle: f64,

    pub x_p_count: u32,
    pub x_n_count: u32,
    pub y_p_count: u32,
    pub y_n_count: u32,
    pub z_p_count: u32,
    pub z_n_count: u32,

    pub acc_x_p_sum: Vector3<f64>,
    pub acc_x_n_sum: Vector3<f64>,
    pub acc_y_p_sum: Vector3<f64>,
    pub acc_y_n_sum: Vector3<f64>,
    pub acc_z_p_sum: Vector3<f64>,
    pub acc_z_n_sum: Vector3<f64>,

    pub gyro_x_p_sum: Vector3<f64>,
    pub gyro_x_n_sum: Vector3<f64>,
    pub gyro_y_p_sum: Vector3<f64>,
    pub gyro_y_n_sum: Vector3<f64>,
    pub gyro_z_p_sum: Vector3<f64>,
    pub gyro_z_n_sum: Vector3<f64>,

    pub x_p_variance_count: u32,
    pub x_n_variance_count: u32,
    pub y_p_variance_count: u32,
    pub y_n_variance_count: u32,
    pub z_p_variance_count: u32,
    pub z_n_variance_count: u32,

    pub acc_x_p_variance_sum: Vector3<f64>,
    pub acc_x_n_variance_sum: Vector3<f64>,
    pub acc_y_p_variance_sum: Vector3<f64>,
    pub acc_y_n_variance_sum: Vector3<f64>,
    pub acc_z_p_variance_sum: Vector3<f64>,
    pub acc_z_n_variance_sum: Vector3<f64>,

    pub gyro_x_p_variance_sum: Vector3<f64>,
    pub gyro_x_n_variance_sum: Vector3<f64>,
    pub gyro_y_p_variance_sum: Vector3<f64>,
    pub gyro_y_n_variance_sum: Vector3<f64>,
    pub gyro_z_p_variance_sum: Vector3<f64>,
    pub gyro_z_n_variance_sum: Vector3<f64>,

    pub x_rotation_count: u32,
    pub y_rotation_count: u32,
    pub z_rotation_count: u32,

    pub x_rotation_start_time_ms: f64,
    pub y_rotation_start_time_ms: f64,
    pub z_rotation_start_time_ms: f64,

    pub x_rotation_end_time_ms: f64,
    pub y_rotation_end_time_ms: f64,
    pub z_rotation_end_time_ms: f64,

    pub acc_x_rotation_sum: Vector3<f64>,
    pub acc_y_rotation_sum: Vector3<f64>,
    pub acc_z_rotation_sum: Vector3<f64>,

    pub gyro_x_rotation_sum: Vector3<f64>,
    pub gyro_y_rotation_sum: Vector3<f64>,
    pub gyro_z_rotation_sum: Vector3<f64>,
}

impl CalibratorInner {
    pub fn new(gravity: Option<f64>, expected_angle: Option<f64>) -> Self {
        CalibratorInner {
            gravity: gravity.unwrap_or(9.81),
            expected_angle: expected_angle.unwrap_or(-360.0),

            x_p_count: 0,
            x_n_count: 0,
            y_p_count: 0,
            y_n_count: 0,
            z_p_count: 0,
            z_n_count: 0,

            acc_x_p_sum: Vector3::zeros(),
            acc_x_n_sum: Vector3::zeros(),
            acc_y_p_sum: Vector3::zeros(),
            acc_y_n_sum: Vector3::zeros(),
            acc_z_p_sum: Vector3::zeros(),
            acc_z_n_sum: Vector3::zeros(),

            gyro_x_p_sum: Vector3::zeros(),
            gyro_x_n_sum: Vector3::zeros(),
            gyro_y_p_sum: Vector3::zeros(),
            gyro_y_n_sum: Vector3::zeros(),
            gyro_z_p_sum: Vector3::zeros(),
            gyro_z_n_sum: Vector3::zeros(),

            x_p_variance_count: 0,
            x_n_variance_count: 0,
            y_p_variance_count: 0,
            y_n_variance_count: 0,
            z_p_variance_count: 0,
            z_n_variance_count: 0,

            acc_x_p_variance_sum: Vector3::zeros(),
            acc_x_n_variance_sum: Vector3::zeros(),
            acc_y_p_variance_sum: Vector3::zeros(),
            acc_y_n_variance_sum: Vector3::zeros(),
            acc_z_p_variance_sum: Vector3::zeros(),
            acc_z_n_variance_sum: Vector3::zeros(),

            gyro_x_p_variance_sum: Vector3::zeros(),
            gyro_x_n_variance_sum: Vector3::zeros(),
            gyro_y_p_variance_sum: Vector3::zeros(),
            gyro_y_n_variance_sum: Vector3::zeros(),
            gyro_z_p_variance_sum: Vector3::zeros(),
            gyro_z_n_variance_sum: Vector3::zeros(),

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

    pub fn process_x_p(&mut self, reading: &IMUReading) {
        self.x_p_count += 1;
        self.acc_x_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_x_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_x_p_variance(&mut self, reading: &IMUReading) {
        self.x_p_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_x_p_sum / self.x_p_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_x_p_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_x_p_sum / self.x_p_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_x_p_variance_sum += gyro_diff;
    }

    pub fn process_x_n(&mut self, reading: &IMUReading) {
        self.x_n_count += 1;
        self.acc_x_n_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_x_n_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_x_n_variance(&mut self, reading: &IMUReading) {
        self.x_n_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_x_n_sum / self.x_n_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_x_n_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_x_n_sum / self.x_n_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_x_n_variance_sum += gyro_diff;
    }

    pub fn process_y_p(&mut self, reading: &IMUReading) {
        self.y_p_count += 1;
        self.acc_y_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_y_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_y_p_variance(&mut self, reading: &IMUReading) {
        self.y_p_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_y_p_sum / self.y_p_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_y_p_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_y_p_sum / self.y_p_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_y_p_variance_sum += gyro_diff;
    }

    pub fn process_y_n(&mut self, reading: &IMUReading) {
        self.y_n_count += 1;
        self.acc_y_n_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_y_n_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_y_n_variance(&mut self, reading: &IMUReading) {
        self.y_n_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_y_n_sum / self.y_n_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_y_n_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_y_n_sum / self.y_n_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_y_n_variance_sum += gyro_diff;
    }

    pub fn process_z_p(&mut self, reading: &IMUReading) {
        self.z_p_count += 1;
        self.acc_z_p_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_z_p_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_z_p_variance(&mut self, reading: &IMUReading) {
        self.z_p_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_z_p_sum / self.z_p_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_z_p_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_z_p_sum / self.z_p_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_z_p_variance_sum += gyro_diff;
    }

    pub fn process_z_n(&mut self, reading: &IMUReading) {
        self.z_n_count += 1;
        self.acc_z_n_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_z_n_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_z_n_variance(&mut self, reading: &IMUReading) {
        self.z_n_variance_count += 1;

        let mut acc_diff =
            Vector3::from_row_slice(&reading.acc).cast() - self.acc_z_n_sum / self.z_n_count as f64;
        acc_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.acc_z_n_variance_sum += acc_diff;

        let mut gyro_diff = Vector3::from_row_slice(&reading.gyro).cast()
            - self.gyro_z_n_sum / self.z_n_count as f64;
        gyro_diff.iter_mut().for_each(|x| *x = *x * *x);
        self.gyro_z_n_variance_sum += gyro_diff;
    }

    pub fn process_x_rotation(&mut self, reading: &IMUReading) {
        if self.x_rotation_count == 0 {
            self.x_rotation_start_time_ms = reading.timestamp;
        }
        self.x_rotation_end_time_ms = reading.timestamp;
        self.x_rotation_count += 1;
        self.acc_x_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_x_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_y_rotation(&mut self, reading: &IMUReading) {
        if self.y_rotation_count == 0 {
            self.y_rotation_start_time_ms = reading.timestamp;
        }
        self.y_rotation_end_time_ms = reading.timestamp;
        self.y_rotation_count += 1;
        self.acc_y_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_y_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    pub fn process_z_rotation(&mut self, reading: &IMUReading) {
        if self.z_rotation_count == 0 {
            self.z_rotation_start_time_ms = reading.timestamp;
        }
        self.z_rotation_end_time_ms = reading.timestamp;
        self.z_rotation_count += 1;
        self.acc_z_rotation_sum += Vector3::from_row_slice(&reading.acc).cast();
        self.gyro_z_rotation_sum += Vector3::from_row_slice(&reading.gyro).cast();
    }

    #[allow(non_snake_case)]
    pub fn calculate(mut self) -> Option<CalibrationInfo> {
        // Compute Acceleration Matrix

        // Calculate means from all static phases and stack them into 3x3 matrices
        // Note: Each measurement should be a column
        let U_a_p = Matrix3::from_columns(&[
            self.acc_x_p_sum / self.x_p_count as f64,
            self.acc_y_p_sum / self.y_p_count as f64,
            self.acc_z_p_sum / self.z_p_count as f64,
        ]);

        let U_a_n = Matrix3::from_columns(&[
            self.acc_x_n_sum / self.x_n_count as f64,
            self.acc_y_n_sum / self.y_n_count as f64,
            self.acc_z_n_sum / self.z_n_count as f64,
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
            1.0 / (4.0 * self.gravity * self.gravity) * (U_a_d * U_a_d.transpose()).diagonal();
        let K_a = Matrix3::from_diagonal(&k_a_sq.map(sqrt));

        // Calculate Rotation matrix
        // Eq. 22
        let R_a = K_a.try_inverse()? * (U_a_d / (2.0 * self.gravity));

        // Calculate Gyroscope Matrix

        // Gyro Bias from the static phases of the acc calibration
        // One static phase would be sufficient, but why not use all of them if you have them.
        // Note that this calibration ignores any influences due to the earth rotation.
        let b_g = (self.gyro_x_p_sum
            + self.gyro_y_p_sum
            + self.gyro_z_p_sum
            + self.gyro_x_n_sum
            + self.gyro_y_n_sum
            + self.gyro_z_n_sum)
            / (self.x_p_count
                + self.y_p_count
                + self.z_p_count
                + self.x_n_count
                + self.y_n_count
                + self.z_n_count) as f64;

        // Acceleration sensitivity
        let U_g_p = Matrix3::from_columns(&[
            self.gyro_x_p_sum / self.x_p_count as f64,
            self.gyro_y_p_sum / self.y_p_count as f64,
            self.gyro_z_p_sum / self.z_p_count as f64,
        ]);
        let U_g_a = Matrix3::from_columns(&[
            self.gyro_x_n_sum / self.x_n_count as f64,
            self.gyro_y_n_sum / self.y_n_count as f64,
            self.gyro_z_n_sum / self.z_n_count as f64,
        ]);

        // Eq. 9
        let K_ga = (U_g_p - U_g_a) / (2.0 * self.gravity);

        // Gyroscope Scaling and Rotation

        // First apply partial calibration to remove offset and acceleration influence
        let acc_mat = R_a.try_inverse()? * K_a.try_inverse()?;
        let apply_calibration =
            |acc_sum: &mut Vector3<f64>, gyro_sum: &mut Vector3<f64>, count: u32| {
                *acc_sum = acc_mat * (*acc_sum - count as f64 * b_a);
                *gyro_sum = *gyro_sum - K_ga * *acc_sum - count as f64 * b_g;
            };
        apply_calibration(
            &mut self.acc_x_rotation_sum,
            &mut self.gyro_x_rotation_sum,
            self.x_rotation_count,
        );
        apply_calibration(
            &mut self.acc_y_rotation_sum,
            &mut self.gyro_y_rotation_sum,
            self.y_rotation_count,
        );
        apply_calibration(
            &mut self.acc_z_rotation_sum,
            &mut self.gyro_z_rotation_sum,
            self.z_rotation_count,
        );

        // Integrate gyro readings
        // Eq. 13/14
        let W_s = Matrix3::from_columns(&[
            self.gyro_x_rotation_sum
                / (self.x_rotation_count as f64
                    / ((self.x_rotation_end_time_ms - self.x_rotation_start_time_ms) as f64
                        / 1000.0)),
            self.gyro_y_rotation_sum
                / (self.y_rotation_count as f64
                    / ((self.y_rotation_end_time_ms - self.y_rotation_start_time_ms) as f64
                        / 1000.0)),
            self.gyro_z_rotation_sum
                / (self.z_rotation_count as f64
                    / ((self.z_rotation_end_time_ms - self.z_rotation_start_time_ms) as f64
                        / 1000.0)),
        ]);

        // Eq. 15
        let expected_angles = self.expected_angle * Matrix3::identity();
        let multiplied = W_s * expected_angles.try_inverse()?;

        // Eq. 12
        let k_g_sq = (multiplied * multiplied.transpose()).diagonal();
        let K_g = Matrix3::from_diagonal(&k_g_sq.map(sqrt));

        let R_g = K_g.try_inverse()? * multiplied;

        let mut acc_variance = (self.acc_x_p_variance_sum
            + self.acc_y_p_variance_sum
            + self.acc_z_p_variance_sum
            + self.acc_x_n_variance_sum
            + self.acc_y_n_variance_sum
            + self.acc_z_n_variance_sum)
            / (self.x_p_variance_count
                + self.y_p_variance_count
                + self.z_p_variance_count
                + self.x_n_variance_count
                + self.y_n_variance_count
                + self.z_n_variance_count) as f64;
        acc_variance.iter_mut().for_each(|x| *x = sqrt(*x));

        let mut gyro_variance = (self.gyro_x_p_variance_sum
            + self.gyro_y_p_variance_sum
            + self.gyro_z_p_variance_sum
            + self.gyro_x_n_variance_sum
            + self.gyro_y_n_variance_sum
            + self.gyro_z_n_variance_sum)
            / (self.x_p_variance_count
                + self.y_p_variance_count
                + self.z_p_variance_count
                + self.x_n_variance_count
                + self.y_n_variance_count
                + self.z_n_variance_count) as f64;
        gyro_variance.iter_mut().for_each(|x| *x = sqrt(*x));

        CalibrationInfo::from_raw(
            K_a.cast(),
            R_a.cast(),
            b_a.cast(),
            K_g.cast(),
            R_g.cast(),
            K_ga.cast(),
            b_g.cast(),
            acc_variance.cast(),
            gyro_variance.cast(),
        )
    }
}
