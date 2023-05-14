use nalgebra::{Matrix3, Vector3};

use crate::imu_reading::IMUReading;

#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
// Couldn't figure out how to get rkyv to work
// #[cfg_attr(
//     feature = "rkyv-no-std",
//     derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
// )]
// #[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct CalibrationInfo {
    pub(crate) b_a: Vector3<f32>,
    pub(crate) K_ga: Matrix3<f32>,
    pub(crate) b_g: Vector3<f32>,
    pub(crate) acc_mat: Matrix3<f32>,
    pub(crate) gyro_mat: Matrix3<f32>,
}

impl defmt::Format for CalibrationInfo {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "CalibrationInfo {{\n");
        defmt::write!(
            fmt,
            "    b_a:      [{}, {}, {}],\n",
            self.b_a.x,
            self.b_a.y,
            self.b_a.z
        );
        defmt::write!(
            fmt,
            "    K_ga:     [{}, {}, {},\n",
            self.K_ga.m11,
            self.K_ga.m12,
            self.K_ga.m13
        );
        defmt::write!(
            fmt,
            "               {}, {}, {},\n",
            self.K_ga.m21,
            self.K_ga.m22,
            self.K_ga.m23
        );
        defmt::write!(
            fmt,
            "               {}, {}, {}],\n",
            self.K_ga.m31,
            self.K_ga.m32,
            self.K_ga.m33
        );
        defmt::write!(
            fmt,
            "    b_g:      [{}, {}, {}],\n",
            self.b_g.x,
            self.b_g.y,
            self.b_g.z
        );
        defmt::write!(
            fmt,
            "    acc_mat:  [{}, {}, {},\n",
            self.acc_mat.m11,
            self.acc_mat.m12,
            self.acc_mat.m13
        );
        defmt::write!(
            fmt,
            "               {}, {}, {},\n",
            self.acc_mat.m21,
            self.acc_mat.m22,
            self.acc_mat.m23
        );
        defmt::write!(
            fmt,
            "               {}, {}, {}],\n",
            self.acc_mat.m31,
            self.acc_mat.m32,
            self.acc_mat.m33
        );
        defmt::write!(
            fmt,
            "    gyro_mat: [{}, {}, {},\n",
            self.gyro_mat.m11,
            self.gyro_mat.m12,
            self.gyro_mat.m13
        );
        defmt::write!(
            fmt,
            "               {}, {}, {},\n",
            self.gyro_mat.m21,
            self.gyro_mat.m22,
            self.gyro_mat.m23
        );
        defmt::write!(
            fmt,
            "               {}, {}, {}],\n",
            self.gyro_mat.m31,
            self.gyro_mat.m32,
            self.gyro_mat.m33
        );
        defmt::write!(fmt, "}}");
    }
}

impl CalibrationInfo {
    #[allow(non_snake_case)]
    pub fn new(
        b_a: Vector3<f32>,
        K_ga: Matrix3<f32>,
        b_g: Vector3<f32>,
        acc_mat: Matrix3<f32>,
        gyro_mat: Matrix3<f32>,
    ) -> Self {
        Self {
            b_a,
            K_ga,
            b_g,
            acc_mat,
            gyro_mat,
        }
    }

    #[allow(non_snake_case)]
    pub(crate) fn from_raw(
        K_a: Matrix3<f32>,
        R_a: Matrix3<f32>,
        b_a: Vector3<f32>,
        K_g: Matrix3<f32>,
        R_g: Matrix3<f32>,
        K_ga: Matrix3<f32>,
        b_g: Vector3<f32>,
    ) -> Option<Self> {
        Some(Self {
            b_a,
            K_ga,
            b_g,
            acc_mat: R_a.try_inverse()? * K_a.try_inverse()?,
            gyro_mat: R_g.try_inverse()? * K_g.try_inverse()?,
        })
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

    pub fn apply_calibration(&self, imu_reading: &IMUReading) -> IMUReading {
        let calibrated_acc = self.calibrate_acc(Vector3::from_row_slice(&imu_reading.acc));
        let calibrated_gyr =
            self.calibrate_gyr(Vector3::from_row_slice(&imu_reading.gyro), calibrated_acc);
        IMUReading {
            timestamp: imu_reading.timestamp,
            acc: calibrated_acc.into(),
            gyro: calibrated_gyr.into(),
        }
    }

    // workaround until getting rkyv working
    // WARNING: this is not endian-safe
    pub fn serialize(&self, buffer: &mut [u8; 132]) {
        unsafe {
            (&mut buffer[0..12]).copy_from_slice(core::slice::from_raw_parts(
                self.b_a.as_ptr() as *const u8,
                12,
            ));
            (&mut buffer[12..48]).copy_from_slice(core::slice::from_raw_parts(
                self.K_ga.as_ptr() as *const u8,
                36,
            ));
            (&mut buffer[48..60]).copy_from_slice(core::slice::from_raw_parts(
                self.b_g.as_ptr() as *const u8,
                12,
            ));
            (&mut buffer[60..96]).copy_from_slice(core::slice::from_raw_parts(
                self.acc_mat.as_ptr() as *const u8,
                36,
            ));
            (&mut buffer[96..132]).copy_from_slice(core::slice::from_raw_parts(
                self.gyro_mat.as_ptr() as *const u8,
                36,
            ));
        }
    }

    // workaround until getting rkyv working
    // WARNING: this is not endian-safe
    pub fn deserialize(buffer: [u8; 132]) -> Self {
        unsafe {
            Self {
                b_a: Vector3::from_column_slice(core::slice::from_raw_parts(
                    buffer.as_ptr() as *const f32,
                    3,
                )),
                K_ga: Matrix3::from_column_slice(core::slice::from_raw_parts(
                    buffer[12..48].as_ptr() as *const f32,
                    9,
                )),
                b_g: Vector3::from_column_slice(core::slice::from_raw_parts(
                    buffer[48..60].as_ptr() as *const f32,
                    3,
                )),
                acc_mat: Matrix3::from_column_slice(core::slice::from_raw_parts(
                    buffer[60..96].as_ptr() as *const f32,
                    9,
                )),
                gyro_mat: Matrix3::from_column_slice(core::slice::from_raw_parts(
                    buffer[96..132].as_ptr() as *const f32,
                    9,
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    fn create_cal_info() -> CalibrationInfo {
        CalibrationInfo::from_raw(
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
        )
        .unwrap()
    }

    #[test]
    fn apply_calibration() {
        let cal_info = create_cal_info();

        let imu_reading = IMUReading {
            timestamp: 0.0,
            acc: [-0.0009765625, 0.12249755859375, 0.016357421875],
            gyro: [-0.549618320610687, -5.099236641221374, -1.083969465648855],
        };
        let calibrated = cal_info.apply_calibration(&imu_reading);

        assert_eq!(calibrated.timestamp, 0.0);
        assert_abs_diff_eq!(
            Vector3::from_row_slice(&calibrated.acc),
            Vector3::new(0.01484103, 9.86576754, -0.0160403),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            Vector3::from_row_slice(&calibrated.gyro),
            Vector3::new(0.11598146, -2.59643185, 0.07955285),
            epsilon = 0.0001
        );
    }

    #[test]
    fn serialization() {
        let cal_info = create_cal_info();
        let mut buffer = [0u8; 132];
        cal_info.serialize(&mut buffer);
        let deserialized = CalibrationInfo::deserialize(buffer);
        assert_eq!(cal_info, deserialized);
    }
}
