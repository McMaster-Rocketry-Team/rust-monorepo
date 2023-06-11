use defmt::*;

use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibrator, InteractiveCalibratorState::*,
};
use vlfs::{io_traits::AsyncWriter, Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::{device_manager::prelude::*, files::CALIBRATION_FILE_TYPE, ticker::Ticker},
    device_manager_type,
    driver::{buzzer::Buzzer, imu::IMU, serial::Serial, timer::Timer, debugger::DebuggerEvent},
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
        serial: &mut impl Serial,
        fs: &VLFS<impl Flash, impl Crc>,
        device_manager: device_manager_type!(),
    ) -> Result<(), ()> {
        let timer = device_manager.timer;
        let debugger = device_manager.debugger.clone();
        claim_devices!(device_manager, buzzer, imu);
        // TODO move this to main
        unwrap!(imu.wait_for_power_on().await);
        unwrap!(imu.reset().await);

        debugger.dispatch(DebuggerEvent::CalibrationStart);
        let mut calibrator = InteractiveCalibrator::new(Some(0.05), None, None);
        loop {
            let reading = imu.read().await.map_err(|_| ()).unwrap();
            let next_state = calibrator.process(&reading);
            if let Some(state) = next_state {
                debugger.dispatch(DebuggerEvent::Calibrating(state));
                match state {
                    WaitingStill => self.waiting_still_sound(&mut buzzer, timer).await,
                    State(axis, direction, event) if event != Event::Variance => {
                        self.axis_sound(axis, &mut buzzer, timer).await;
                        self.direction_sound(direction, &mut buzzer, timer).await;
                        self.event_sound(event, &mut buzzer, timer).await;
                    }
                    State(_, _, _) => {}
                    Success => {
                        self.success_sound(&mut buzzer, timer).await;
                        break;
                    }
                    Failure => {
                        self.failure_sound(&mut buzzer, timer).await;
                        break;
                    }
                    Idle => {}
                }

                if let Success = state {
                    let cal_info = calibrator.get_calibration_info().unwrap();
                    info!("{}", cal_info);
                    unwrap!(
                        fs.remove_files(|file_entry| file_entry.file_type == CALIBRATION_FILE_TYPE)
                            .await
                    );
                    let file_id = unwrap!(fs.create_file(CALIBRATION_FILE_TYPE).await);
                    let mut file = unwrap!(fs.open_file_for_write(file_id).await);

                    let mut buffer = [0u8; 156];
                    cal_info.serialize(&mut buffer);
                    unwrap!(file.extend_from_slice(&buffer).await);
                    unwrap!(file.close().await);

                    unwrap!(serial.write(&[1]).await);
                    break;
                } else if let Failure = state {
                    unwrap!(serial.write(&[0]).await);
                    break;
                }
            }
        }

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
                let mut ticker = Ticker::every(timer, 4.0);
                for frequency in (2000..3500).step_by(100) {
                    buzzer.set_frequency(frequency).await;
                    ticker.next().await;
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
            Event::Variance => {}
        }
    }

    async fn success_sound<B: Buzzer, TI: Timer>(&self, buzzer: &mut B, timer: TI) {
        for i in 0..4 {
            buzzer.set_frequency(1000 + i * 250).await;
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;
        }
    }

    async fn failure_sound<B: Buzzer, TI: Timer>(&self, buzzer: &mut B, timer: TI) {
        for i in (0..4).rev() {
            buzzer.set_frequency(1000 + i * 250).await;
            buzzer.set_enable(true).await;
            timer.sleep(50.0).await;
            buzzer.set_enable(false).await;
            timer.sleep(150.0).await;
        }
    }
}
