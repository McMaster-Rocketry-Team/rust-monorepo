#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

use avionics::start_avionics_thread;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::prelude::*;
use virt_drivers::{debugger::create_debugger, serial::create_virtual_serial, sensors::create_sensors};

mod avionics;
mod virt_drivers;

fn main() {
    let (debugger, debugger_rx) = create_debugger();
    let (serial_a,serial_b) = create_virtual_serial();
    let (sensor_tx, imu) = create_sensors();

    start_avionics_thread("./avionics.fl".into(), imu, serial_a, debugger);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(PanOrbitCameraPlugin)
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_system(print_body_velocity)
        .run();
}

fn setup_graphics(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-0.3, 0.15, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera::default(),
    ));
}

fn setup_physics(mut commands: Commands) {
    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(5.0, 0.1, 5.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -0.1, 0.0)));

    /* Create the bouncing ball. */
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Sleeping {
            linear_threshold: -1.0,
            angular_threshold: -1.0,
            sleeping: false,
        })
        .insert(Velocity {
            linvel: Vec3::ZERO,
            angvel: Vec3::ZERO,
        })
        .insert(Collider::cuboid(0.04 / 2.0, 0.02 / 2.0, 0.05 / 2.0))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.1, 0.0)));
}

fn print_body_velocity(q: Query<(&Velocity, &Transform)>) {
    for (vel, pos) in q.iter() {
        println!("velocity: {}", vel.linvel);
        println!("gyro: {}", vel.angvel);
        println!("position: {}", pos.translation);
        println!("orientation: {}", pos.rotation);
    }
}