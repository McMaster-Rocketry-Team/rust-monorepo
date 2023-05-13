use heapless::Vec;
use nalgebra::Vector3;

pub struct SignalRegions<const N:usize, const M:usize> {
    pub acc_x_p: Vec<Vector3<f32>, N>,
    pub acc_x_a: Vec<Vector3<f32>, N>,
    pub acc_y_p: Vec<Vector3<f32>, N>,
    pub acc_y_a: Vec<Vector3<f32>, N>,
    pub acc_z_p: Vec<Vector3<f32>, N>,
    pub acc_z_a: Vec<Vector3<f32>, N>,
    pub gyr_x_p: Vec<Vector3<f32>, N>,
    pub gyr_x_a: Vec<Vector3<f32>, N>,
    pub gyr_y_p: Vec<Vector3<f32>, N>,
    pub gyr_y_a: Vec<Vector3<f32>, N>,
    pub gyr_z_p: Vec<Vector3<f32>, N>,
    pub gyr_z_a: Vec<Vector3<f32>, N>,

    pub acc_x_rot: Vec<Vector3<f32>, M>,
    pub acc_y_rot: Vec<Vector3<f32>, M>,
    pub acc_z_rot: Vec<Vector3<f32>, M>,
    pub gyr_x_rot: Vec<Vector3<f32>, M>,
    pub gyr_y_rot: Vec<Vector3<f32>, M>,
    pub gyr_z_rot: Vec<Vector3<f32>, M>,
}

type A = SignalRegions<100, 1000>;