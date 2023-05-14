# Ferraris Calibration

> A rust crate to calibrate 6 DOF IMUs

## Features

- `#[no_std]` support
- Doesn't need a memory allocator
- Works on streaming sensor data

## Overview

This crate ports and improves on the python [imucal](https://github.com/mad-lab-fau/imucal) package, which implements [Ferraris Calibration](https://www.researchgate.net/publication/245080041_Calibration_of_three-axial_rate_gyros_without_angular_velocity_standards).

## Examples

- [Creating Calibration](./src/calibrator.rs#L449)
- [Applying Calibration](./src/calibration_info.rs#L82)