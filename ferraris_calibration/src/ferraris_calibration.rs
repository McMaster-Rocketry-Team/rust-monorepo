use core::iter::zip;

use crate::{calibration_info::CalibrationInfo, signal_regions::SignalRegions};
#[allow(unused_imports)]
use micromath::F32Ext;
use nalgebra::{Matrix3, Vector3};

#[allow(non_snake_case)]
pub fn ferraris_calibration<const N: usize, const M: usize>(
    mut regions: SignalRegions<N, M>,
    sampling_rate_hz: f32,
    gravity: Option<f32>,
    expected_angle: Option<f32>,
) -> CalibrationInfo {
    let gravity = gravity.unwrap_or(9.81);
    let expected_angle = expected_angle.unwrap_or(-360.0);

    //////////////////////////////////////////////////////////////////////////
    // Compute Acceleration Matrix

    // Calculate means from all static phases and stack them into 3x3 matrices
    // Note: Each measurement should be a column
    let U_a_p = Matrix3::from_columns(&[
        regions.acc_x_p.iter().sum::<Vector3<f32>>() / regions.acc_x_p.len() as f32,
        regions.acc_y_p.iter().sum::<Vector3<f32>>() / regions.acc_y_p.len() as f32,
        regions.acc_z_p.iter().sum::<Vector3<f32>>() / regions.acc_z_p.len() as f32,
    ]);

    let U_a_n = Matrix3::from_columns(&[
        regions.acc_x_a.iter().sum::<Vector3<f32>>() / regions.acc_x_a.len() as f32,
        regions.acc_y_a.iter().sum::<Vector3<f32>>() / regions.acc_y_a.len() as f32,
        regions.acc_z_a.iter().sum::<Vector3<f32>>() / regions.acc_z_a.len() as f32,
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
    let k_a_sq = 1.0 / (4.0 * gravity * gravity) * (U_a_d * U_a_d.transpose()).diagonal();
    let K_a = Matrix3::from_diagonal(&k_a_sq.map(|x| x.sqrt()));

    // Calculate Rotation matrix
    // Eq. 22
    let R_a = K_a.try_inverse().unwrap() * (U_a_d / (2.0 * gravity));

    //////////////////////////////////////////////////////////////////////////////////////
    // Calculate Gyroscope Matrix

    // Gyro Bias from the static phases of the acc calibration
    // One static phase would be sufficient, but why not use all of them if you have them.
    // Note that this calibration ignores any influences due to the earth rotation.

    let b_g = (regions.gyr_x_p.iter().sum::<Vector3<f32>>()
        + regions.gyr_y_p.iter().sum::<Vector3<f32>>()
        + regions.gyr_z_p.iter().sum::<Vector3<f32>>()
        + regions.gyr_x_a.iter().sum::<Vector3<f32>>()
        + regions.gyr_y_a.iter().sum::<Vector3<f32>>()
        + regions.gyr_z_a.iter().sum::<Vector3<f32>>())
        / (regions.gyr_x_p.len() * 6) as f32;

    // Acceleration sensitivity

    // Note: Each measurement should be a column
    let U_g_p = Matrix3::from_columns(&[
        regions.gyr_x_p.iter().sum::<Vector3<f32>>() / regions.gyr_x_p.len() as f32,
        regions.gyr_y_p.iter().sum::<Vector3<f32>>() / regions.gyr_y_p.len() as f32,
        regions.gyr_z_p.iter().sum::<Vector3<f32>>() / regions.gyr_z_p.len() as f32,
    ]);
    let U_g_a = Matrix3::from_columns(&[
        regions.gyr_x_a.iter().sum::<Vector3<f32>>() / regions.gyr_x_a.len() as f32,
        regions.gyr_y_a.iter().sum::<Vector3<f32>>() / regions.gyr_y_a.len() as f32,
        regions.gyr_z_a.iter().sum::<Vector3<f32>>() / regions.gyr_z_a.len() as f32,
    ]);

    // Eq. 9
    let K_ga = (U_g_p - U_g_a) / (2.0 * gravity);

    // Gyroscope Scaling and Rotation

    // First apply partial calibration to remove offset and acc influence
    let cal_info = CalibrationInfo::new(
        K_a,
        R_a,
        b_a,
        Matrix3::identity(),
        Matrix3::identity(),
        K_ga,
        b_g,
    );
    let apply_calibraion = |(acc, gyro): (&mut Vector3<f32>, &mut Vector3<f32>)| {
        *acc = cal_info.calibrate_acc(*acc);
        *gyro = cal_info.calibrate_gyr_offsets(*gyro, *acc);
    };
    zip(regions.acc_x_rot.iter_mut(), regions.gyr_x_rot.iter_mut()).for_each(apply_calibraion);
    zip(regions.acc_y_rot.iter_mut(), regions.gyr_y_rot.iter_mut()).for_each(apply_calibraion);
    zip(regions.acc_z_rot.iter_mut(), regions.gyr_z_rot.iter_mut()).for_each(apply_calibraion);

    // Integrate gyro readings
    // Eq. 13/14
    let W_s = Matrix3::from_columns(&[
        regions.gyr_x_rot.iter().sum::<Vector3<f32>>() / sampling_rate_hz,
        regions.gyr_y_rot.iter().sum::<Vector3<f32>>() / sampling_rate_hz,
        regions.gyr_z_rot.iter().sum::<Vector3<f32>>() / sampling_rate_hz,
    ]);

    // Eq. 15
    let expected_angles = expected_angle * Matrix3::identity();
    let multiplied = W_s * expected_angles.try_inverse().unwrap();

    // Eq. 12
    let k_g_sq = (multiplied * multiplied.transpose()).diagonal();
    let K_g = Matrix3::from_diagonal(&k_g_sq.map(|x| x.sqrt()));

    let R_g = K_g.try_inverse().unwrap() * multiplied;

    return CalibrationInfo::new(K_a, R_a, b_a, K_g, R_g, K_ga, b_g);
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use heapless::Vec;
    use std::path::Path;

    use super::*;

    fn read_file<const N: usize, P: AsRef<Path>>(
        path: P,
    ) -> (Vec<Vector3<f32>, N>, Vec<Vector3<f32>, N>) {
        let mut acc_list: Vec<Vector3<f32>, N> = Vec::new();
        let mut gyr_list: Vec<Vector3<f32>, N> = Vec::new();

        for line in csv::Reader::from_path(path).unwrap().records() {
            let record = line.unwrap();
            acc_list
                .push(Vector3::new(
                    record[1].parse::<f32>().unwrap(),
                    record[2].parse::<f32>().unwrap(),
                    record[3].parse::<f32>().unwrap(),
                ))
                .unwrap();
            gyr_list
                .push(Vector3::new(
                    record[4].parse::<f32>().unwrap(),
                    record[5].parse::<f32>().unwrap(),
                    record[6].parse::<f32>().unwrap(),
                ))
                .unwrap();
        }

        (acc_list, gyr_list)
    }

    #[test]
    fn calculate_calibration() {
        let (acc_x_p, gyr_x_p) = read_file::<100, _>("./calibration_data/x_plus.csv");
        let (acc_x_a, gyr_x_a) = read_file::<100, _>("./calibration_data/x_minus.csv");
        let (acc_y_p, gyr_y_p) = read_file::<100, _>("./calibration_data/y_plus.csv");
        let (acc_y_a, gyr_y_a) = read_file::<100, _>("./calibration_data/y_minus.csv");
        let (acc_z_p, gyr_z_p) = read_file::<100, _>("./calibration_data/z_plus.csv");
        let (acc_z_a, gyr_z_a) = read_file::<100, _>("./calibration_data/z_minus.csv");

        let (acc_x_rot, gyr_x_rot) = read_file::<1000, _>("./calibration_data/gyro_x.csv");
        let (acc_y_rot, gyr_y_rot) = read_file::<1000, _>("./calibration_data/gyro_y.csv");
        let (acc_z_rot, gyr_z_rot) = read_file::<1000, _>("./calibration_data/gyro_z.csv");

        let regions = SignalRegions {
            acc_x_p,
            gyr_x_p,
            acc_x_a,
            gyr_x_a,
            acc_y_p,
            gyr_y_p,
            acc_y_a,
            gyr_y_a,
            acc_z_p,
            gyr_z_p,
            acc_z_a,
            gyr_z_a,
            acc_x_rot,
            gyr_x_rot,
            acc_y_rot,
            gyr_y_rot,
            acc_z_rot,
            gyr_z_rot,
        };

        let cal_info = ferraris_calibration(regions, 100.0, None, None);

        assert_abs_diff_eq!(
            cal_info.K_a,
            Matrix3::new(
                0.012853398147689652,
                0.0,
                0.0,
                0.0,
                0.012769395862045202,
                0.0,
                0.0,
                0.0,
                0.01295375876988152
            ),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            cal_info.R_a,
            Matrix3::new(
                -0.9982048218132239,
                -0.04173990717830368,
                0.042952460436817545,
                -0.05027316484209185,
                0.9965297973518218,
                0.06633982127422965,
                -0.00901768776318074,
                0.05922375444149138,
                -0.9982040013029699
            ),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            cal_info.b_a,
            Vector3::new(
                0.004518432617187498,
                -0.003014831542968749,
                0.008571166992187504
            ),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            cal_info.K_g,
            Matrix3::new(
                0.07226674840488502,
                0.0,
                0.0,
                0.0,
                0.07780080582696988,
                0.0,
                0.0,
                0.0,
                0.07584465025863281
            ),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            cal_info.R_g,
            Matrix3::new(
                -0.997511945745026,
                -0.049254568455091755,
                0.05043714486640675,
                -0.049747820906859336,
                0.9948824330521149,
                0.08794372472964161,
                0.012426077603985218,
                0.10064555528672757,
                -0.9948447440663323
            ),
            epsilon = 0.0001
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
                -0.00015173798351891894,
                -0.002610671460030661,
                -0.0011749966928901107
            ),
            epsilon = 0.0001
        );
        assert_abs_diff_eq!(
            cal_info.b_g,
            Vector3::new(
                -0.6039185750636132,
                0.16660305343511447,
                -0.36804071246819337
            ),
            epsilon = 0.0001
        )
    }
}
