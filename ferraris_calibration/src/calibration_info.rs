use nalgebra::{Matrix3, Vector3};

use crate::imu_reading::IMUReading;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct CalibrationInfo {
    pub(crate) K_a: Matrix3<f32>,
    pub(crate) R_a: Matrix3<f32>,
    pub(crate) b_a: Vector3<f32>,
    pub(crate) K_g: Matrix3<f32>,
    pub(crate) R_g: Matrix3<f32>,
    pub(crate) K_ga: Matrix3<f32>,
    pub(crate) b_g: Vector3<f32>,
    pub(crate) acc_mat: Matrix3<f32>,
    pub(crate) gyro_mat: Matrix3<f32>,
}

impl CalibrationInfo {
    pub fn new(
        K_a: Matrix3<f32>,
        R_a: Matrix3<f32>,
        b_a: Vector3<f32>,
        K_g: Matrix3<f32>,
        R_g: Matrix3<f32>,
        K_ga: Matrix3<f32>,
        b_g: Vector3<f32>,
    ) -> Self {
        Self {
            K_a,
            R_a,
            b_a,
            K_g,
            R_g,
            K_ga,
            b_g,
            acc_mat: R_a.try_inverse().unwrap() * K_a.try_inverse().unwrap(),
            gyro_mat: R_g.try_inverse().unwrap() * K_g.try_inverse().unwrap(),
        }
    }

    #[inline(always)]
    pub(crate) fn calibrate_acc(&self, acc: Vector3<f32>) -> Vector3<f32> {
        self.acc_mat * (acc - self.b_a)
    }

    #[inline(always)]
    pub(crate) fn calibrate_gyr_offsets(
        &self,
        gyr: Vector3<f32>,
        calibrated_acc: Vector3<f32>,
    ) -> Vector3<f32> {
        let d_ga: Vector3<f32> = self.K_ga * calibrated_acc;
        gyr - d_ga - self.b_g
    }

    #[inline(always)]
    pub(crate) fn calibrate_gyr(
        &self,
        gyr: Vector3<f32>,
        calibrated_acc: Vector3<f32>,
    ) -> Vector3<f32> {
        self.gyro_mat * self.calibrate_gyr_offsets(gyr, calibrated_acc)
    }

    pub fn apply_calibration(&self, imu_reading: IMUReading) -> IMUReading {
        let calibrated_acc = self.calibrate_acc(imu_reading.acc);
        let calibrated_gyr = self.calibrate_gyr(imu_reading.gyro, calibrated_acc);
        IMUReading {
            timestamp: imu_reading.timestamp,
            acc: calibrated_acc,
            gyro: calibrated_gyr,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn apply_calibration() {
        let cal_info = CalibrationInfo::new(
            Matrix3::new(
                0.012852893468065155,
                0.0,
                0.0,
                0.0,
                0.01276796345665728,
                0.0,
                0.0,
                0.0,
                0.012956898344887247,
            ),
            Matrix3::new(
                -0.9982001341879744,
                -0.04184039113973842,
                0.04296363318416703,
                -0.05030834983924777,
                0.9965147548118217,
                0.06653881107133168,
                -0.009156102816376755,
                0.05916383747237993,
                -0.9982062943684327,
            ),
            Vector3::new(
                0.004528216632726512,
                -0.0030060190520344726,
                0.008588824388634321,
            ),
            Matrix3::new(
                2.0171704132104065,
                0.0,
                0.0,
                0.0,
                2.040499421219369,
                0.0,
                0.0,
                0.0,
                2.0514830144368674,
            ),
            Matrix3::new(
                -0.997448332960245,
                -0.053713632552986966,
                0.047028382417333085,
                -0.04741957727060594,
                0.9945185311580211,
                0.09318945688633039,
                0.015149308073338164,
                0.10330177184373561,
                -0.9945346863729007,
            ),
            Matrix3::new(
                -4.933754934137833e-05,
                0.0003289606112091641,
                5.16460502375119e-05,
                -1.7954920534761e-05,
                -0.0005460548224766581,
                8.63850088197551e-05,
                -0.000190721724602239,
                -0.001251107336749236,
                -0.00123847028316377,
            ),
            Vector3::new(
                -0.608373767680656,
                0.17122412951449306,
                -0.36269792946025897,
            ),
        );

        let imu_reading = IMUReading {
            timestamp: 0,
            acc: Vector3::new(-0.0009765625, 0.12249755859375, 0.016357421875),
            gyro: Vector3::new(-0.549618320610687, -5.099236641221374, -1.083969465648855),
        };
        let calibrated = cal_info.apply_calibration(imu_reading);

        assert_eq!(calibrated.timestamp, 0);
        assert_abs_diff_eq!(
            calibrated.acc,
            Vector3::new(0.01484103, 9.86576754, -0.0160403),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            calibrated.gyro,
            Vector3::new(0.11598146, -2.59643185, 0.07955285),
            epsilon = 0.0001
        );
    }
}
