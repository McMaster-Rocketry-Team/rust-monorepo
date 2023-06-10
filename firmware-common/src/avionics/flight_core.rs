use core::f32::consts::FRAC_PI_2;
use core::ops::Mul;

use crate::common::moving_average::NoSumSMA;
use crate::common::sensor_snapshot::SensorSnapshot;
use eskf::ESKF;
use ferraris_calibration::IMUReading;
use heapless::Deque;
use nalgebra::Matrix3;
use nalgebra::Unit;
use nalgebra::UnitQuaternion;
use nalgebra::Vector3;

use super::baro_reading_filter::BaroFilterOutput;
use super::baro_reading_filter::BaroReadingFilter;

pub enum FlightCoreState {
    Armed {
        // 500ms history
        snapshot_history: Deque<SensorSnapshot, 100>,
        acc_y_moving_average: NoSumSMA<f32, f32, 4>,
    },
    PowerAscend {
        acc_mag_moving_average: NoSumSMA<f32, f32, 4>,
    },
    Coast {},
    DrogueChuteOpen {},
}

impl FlightCoreState {
    pub fn new() -> Self {
        Self::Armed {
            snapshot_history: Deque::new(),
            acc_y_moving_average: NoSumSMA::new(0.0),
        }
    }

    pub fn is_in_air(&self) -> bool {
        match self {
            Self::Armed { .. } => false,
            _ => true,
        }
    }
}

// Assumes all the sensors are working
pub struct FlightCore {
    state: FlightCoreState,
    launch_snapshot: Option<SensorSnapshot>,
    snapshot_before_launch: Option<SensorSnapshot>,
    mounting_angle_compensation_quat: UnitQuaternion<f32>,
    last_snapshot: Option<SensorSnapshot>,
    eskf: ESKF,
    baro_filter: BaroReadingFilter,
}

impl FlightCore {
    pub fn new(rocket_upright_imu_reading: IMUReading) -> Self {
        let upright_gravity_vector = Vector3::from(rocket_upright_imu_reading.acc);
        let sky_vector = -upright_gravity_vector.normalize();
        let plus_y_vector = Vector3::<f32>::new(0.0, 1.0, 0.0);
        Self {
            state: FlightCoreState::new(),
            launch_snapshot: None,
            snapshot_before_launch: None,
            mounting_angle_compensation_quat: UnitQuaternion::rotation_between(
                &sky_vector,
                &plus_y_vector,
            )
            .unwrap(),
            last_snapshot: None,
            eskf: eskf::Builder::new()
                // TODO variance
                // .acceleration_variance(variance)
                // .rotation_variance(variance)
                .initial_covariance(1e-1)
                .build(),
            baro_filter: BaroReadingFilter::new(),
        }
    }

    // Designed to run at 200hz
    pub async fn tick(&mut self, mut snapshot: SensorSnapshot) {
        if self.last_snapshot.is_none() {
            self.last_snapshot = Some(snapshot.clone());
            return;
        }

        let dt = snapshot.imu_reading.timestamp
            - self.last_snapshot.as_ref().unwrap().imu_reading.timestamp;

        // apply mounting angle compensation
        let acc = self
            .mounting_angle_compensation_quat
            .mul(&Vector3::from(snapshot.imu_reading.acc));
        snapshot.imu_reading.acc = acc.clone().into();

        let gyro = self
            .mounting_angle_compensation_quat
            .mul(&Vector3::from(snapshot.imu_reading.gyro));
        snapshot.imu_reading.gyro = gyro.clone().into();

        if self.state.is_in_air() {
            self.eskf.predict(
                y_up_to_z_up(acc.clone()),
                y_up_to_z_up(gyro.clone()),
                (dt / 1000.0) as f32,
            );

            if let BaroFilterOutput {
                should_ignore: false,
                baro_reading,
            } = self.baro_filter.feed(&snapshot.baro_reading)
            {
                self.eskf
                    .observe_height(baro_reading.altitude(), 1.0)
                    .unwrap(); // TODO: variance
            }
        }

        match &mut self.state {
            FlightCoreState::Armed {
                snapshot_history,
                acc_y_moving_average,
            } => {
                acc_y_moving_average.add_sample(snapshot.imu_reading.acc[1]);

                if snapshot_history.is_full() {
                    snapshot_history.pop_front();
                }
                snapshot_history.push_back(snapshot.clone()).unwrap();

                // launch detection
                if snapshot_history.is_full() && acc_y_moving_average.get_average() < -50.0 {
                    // backtrack 500ms to calculate launch angle
                    let snapshot_before_launch = snapshot_history.front().unwrap();
                    self.snapshot_before_launch = Some(snapshot_before_launch.clone());

                    let launch_vector =
                        -Vector3::from(snapshot_before_launch.imu_reading.acc).normalize();
                    let sky_vector = Vector3::<f32>::new(0.0, 1.0, 0.0);
                    let orientation =
                        UnitQuaternion::rotation_between(&sky_vector, &launch_vector).unwrap();
                    self.eskf
                        .observe_orientation(quat_y_up_to_z_up(orientation), Matrix3::zeros())
                        .unwrap();

                    for (prev_snapshot, snapshot) in
                        snapshot_history.iter().zip(snapshot_history.iter().skip(1))
                    {
                        self.eskf.predict(
                            y_up_to_z_up(snapshot.imu_reading.acc.into()),
                            y_up_to_z_up(snapshot.imu_reading.gyro.into()),
                            ((snapshot.timestamp - prev_snapshot.timestamp) / 1000.0) as f32,
                        );
                    }

                    self.state = FlightCoreState::PowerAscend {
                        acc_mag_moving_average: NoSumSMA::new(0.0),
                    };
                }
            }
            FlightCoreState::PowerAscend {
                acc_mag_moving_average,
            } => {
                acc_mag_moving_average
                    .add_sample(Vector3::from(snapshot.imu_reading.acc).magnitude());

                // coast detection
                if acc_mag_moving_average.is_full() && acc_mag_moving_average.get_average() < 10.0 {
                    self.state = FlightCoreState::Coast {};
                }
            }
            FlightCoreState::Coast {} => {
                // apogee detection
                if z_up_to_y_up(self.eskf.velocity).y <= 0.0
                    && snapshot.timestamp - self.snapshot_before_launch.as_ref().unwrap().timestamp
                        > 4000.0
                {
                    self.state = FlightCoreState::DrogueChuteOpen {};
                }
            }
            FlightCoreState::DrogueChuteOpen {} => {}
        }

        self.last_snapshot = Some(snapshot);
    }
}

// The coordinate system used by the flight core and visualizer is Y up, Z forward, X right:
//     Y
//     |
//     |
//     |_______ X
//    /
//   /
//  Z

// The coordinate system used by eskf is Z up, Y backward, X right:
//     Z    Y
//     |  /
//     | /
//     |_______ X

fn y_up_to_z_up(vector: Vector3<f32>) -> Vector3<f32> {
    Vector3::new(vector.x, -vector.z, vector.y)
}

fn z_up_to_y_up(vector: Vector3<f32>) -> Vector3<f32> {
    Vector3::new(vector.x, vector.z, -vector.y)
}

fn quat_y_up_to_z_up(q_orig: UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    let rot_quat = UnitQuaternion::from_axis_angle(&Unit::new_normalize(Vector3::x()), -FRAC_PI_2);
    rot_quat * q_orig
}

fn quat_z_up_to_y_up(q_target: UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    let rot_quat = UnitQuaternion::from_axis_angle(&Unit::new_normalize(Vector3::x()), -FRAC_PI_2);
    rot_quat * q_target
}
