use defmt::*;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibrator, InteractiveCalibratorState,
    InteractiveCalibratorState::*,
};
use futures::future::join;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::device_manager::prelude::*,
    device_manager_type,
    driver::{buzzer::Buzzer, imu::IMU, serial::Serial, timer::Timer},
};

pub struct Calibrate {}

impl Calibrate {
    pub fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x5
    }

    pub async fn start(
        &self,
        _serial: &mut impl Serial,
        _vlfs: &VLFS<impl Flash, impl Crc>,
        device_manager: device_manager_type!(),
    ) -> Result<(), ()> {
        let timer = device_manager.timer;
        claim_devices!(device_manager, buzzer, imu);
        let state_event = Signal::<CriticalSectionRawMutex, InteractiveCalibratorState>::new();

        let sound_fut = async {
            loop {
                match state_event.wait().await {
                    WaitingStill => self.waiting_still_sound(&mut buzzer, timer).await,
                    State(axis, direction, event) => {
                        self.axis_sound(axis, &mut buzzer, timer).await;
                        self.direction_sound(direction, &mut buzzer, timer).await;
                        self.event_sound(event, &mut buzzer, timer).await;
                    }
                    Complete => {
                        self.complete_sound(&mut buzzer, timer).await;
                        break;
                    }
                    Idle => {}
                }
            }
        };

        let calibration_fut = async {
            let mut calibrator = InteractiveCalibrator::default();
            loop {
                let reading = imu.read().await.map_err(|_| ()).unwrap();
                let next_state = calibrator.process(&reading);
                if let Some(state) = next_state {
                    state_event.signal(state);
                    if let Complete = state {
                        info!("{}", calibrator.calculate());
                        break;
                    }
                }
            }
        };

        join(sound_fut, calibration_fut).await;

        Ok(())
    }

    async fn waiting_still_sound(&self, buzzer: &mut impl Buzzer, timer: impl Timer) {
        for _ in 0..2 {
            buzzer.set_frequency(1000).await;
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;

            buzzer.set_frequency(1250).await;
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;
        }
    }

    async fn axis_sound(&self, axis: Axis, buzzer: &mut impl Buzzer, timer: impl Timer) {
        buzzer.set_frequency(2700).await;
        let beep_count = match axis {
            Axis::X => 1,
            Axis::Y => 2,
            Axis::Z => 3,
        };
        for _ in 0..beep_count {
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;
        }

        timer.sleep(250.0).await;
    }

    async fn direction_sound(
        &self,
        direction: Direction,
        buzzer: &mut impl Buzzer,
        timer: impl Timer,
    ) {
        match direction {
            Direction::Plus => {
                buzzer.set_frequency(2000).await;
                buzzer.set_enable(true).await;
                timer.sleep(50.0).await;
                buzzer.set_enable(false).await;

                timer.sleep(150.0).await;

                buzzer.set_frequency(3500).await;
                buzzer.set_enable(true).await;
                timer.sleep(50.0).await;
                buzzer.set_enable(false).await;
            }
            Direction::Minus => {
                buzzer.set_frequency(3500).await;
                buzzer.set_enable(true).await;
                timer.sleep(50.0).await;
                buzzer.set_enable(false).await;

                timer.sleep(150.0).await;

                buzzer.set_frequency(2000).await;
                buzzer.set_enable(true).await;
                timer.sleep(50.0).await;
                buzzer.set_enable(false).await;
            }
            Direction::Rotation => {
                buzzer.set_enable(true).await;
                for frequency in (2000..3500).step_by(100) {
                    buzzer.set_frequency(frequency).await;
                    timer.sleep(4.0).await; // TODO use ticker
                }
                buzzer.set_enable(false).await;
            }
        }
        timer.sleep(400.0).await;
    }

    async fn event_sound(&self, event: Event, buzzer: &mut impl Buzzer, timer: impl Timer) {
        buzzer.set_frequency(1500).await;
        match event {
            Event::Start => {
                buzzer.set_enable(true).await;
                timer.sleep(25.0).await;
                buzzer.set_enable(false).await;

                timer.sleep(75.0).await;

                buzzer.set_enable(true).await;
                timer.sleep(25.0).await;
                buzzer.set_enable(false).await;
            }
            Event::End => {
                buzzer.set_enable(true).await;
                timer.sleep(250.0).await;
                buzzer.set_enable(false).await;
            }
        }
    }

    async fn complete_sound<B: Buzzer, TI: Timer>(&self, buzzer: &mut B, timer: TI) {
        for i in 0..4 {
            buzzer.set_frequency(1000 + i * 250).await;
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;
        }
    }
}
