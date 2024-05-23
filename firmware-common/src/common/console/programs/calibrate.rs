use defmt::*;

use embedded_hal_async::delay::DelayNs;
use ferraris_calibration::interactive_calibrator::{
    Axis, Direction, Event, InteractiveCalibrator, InteractiveCalibratorState::*,
};
use vlfs::{io_traits::AsyncWriter, Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::{
        console::console_program::ConsoleProgram, device_manager::prelude::*,
        files::CALIBRATION_FILE_TYPE, ticker::Ticker,
    },
    device_manager_type,
    driver::{buzzer::Buzzer, debugger::DebuggerTargetEvent, imu::IMU, serial::Serial},
};

pub struct Calibrate<'a, F: Flash, C: Crc, D: DelayNs + Copy> {
    vlfs: &'a VLFS<F, C>,
    delay: D,
}

impl<'a, F: Flash, C: Crc, D: DelayNs + Copy> Calibrate<'a, F, C, D> {
    pub fn new(vlfs: &'a VLFS<F, C>, delay: D) -> Self {
        Self { vlfs, delay }
    }

    async fn waiting_still_sound(&self, buzzer: &mut impl Buzzer) {
        let mut delay = self.delay;
        for _ in 0..2 {
            buzzer.play(1000, 50).await;
            delay.delay_ms(150).await;
            buzzer.play(1250, 50).await;
            delay.delay_ms(150).await;
        }
    }

    async fn axis_sound(&self, axis: Axis, buzzer: &mut impl Buzzer) {
        let mut delay = self.delay;
        let beep_count = match axis {
            Axis::X => 1,
            Axis::Y => 2,
            Axis::Z => 3,
        };
        for _ in 0..beep_count {
            buzzer.play(2700, 50).await;
            delay.delay_ms(150).await;
        }

        delay.delay_ms(250).await;
    }

    async fn direction_sound(&self, direction: Direction, buzzer: &mut impl Buzzer) {
        let mut delay = self.delay;
        match direction {
            Direction::Plus => {
                buzzer.play(2000, 50).await;
                delay.delay_ms(150).await;
                buzzer.play(3500, 50).await;
            }
            Direction::Minus => {
                buzzer.play(3500, 50).await;
                delay.delay_ms(150).await;
                buzzer.play(2000, 50).await;
            }
            Direction::Rotation => {
                for frequency in (2000..3500).step_by(100) {
                    buzzer.play(frequency, 4).await;
                }
            }
        }
        delay.delay_ms(400).await;
    }

    async fn event_sound(&self, event: Event, buzzer: &mut impl Buzzer) {
        let mut delay = self.delay;
        match event {
            Event::Start => {
                buzzer.play(1500, 25).await;
                delay.delay_ms(75).await;
                buzzer.play(1500, 25).await;
            }
            Event::End => {
                buzzer.play(1500, 250).await;
            }
            Event::Variance => {}
        }
    }

    async fn success_sound<B: Buzzer>(&self, buzzer: &mut B) {
        let mut delay = self.delay;
        for i in 0..4 {
            buzzer.play(1000 + i * 250, 50).await;
            delay.delay_ms(150).await;
        }
    }

    async fn failure_sound<B: Buzzer>(&self, buzzer: &mut B) {
        let mut delay = self.delay;
        for i in (0..4).rev() {
            buzzer.play(1000 + i * 250, 50).await;
            delay.delay_ms(150).await;
        }
    }
}

impl<'a, F: Flash, C: Crc, D: DelayNs + Copy> ConsoleProgram for Calibrate<'a, F, C, D> {
    fn id(&self) -> u64 {
        0x05
    }

    async fn run(&mut self, serial: &mut impl Serial, device_manager: device_manager_type!()) {
        let delay = self.delay;
        let debugger = device_manager.debugger.clone();
        claim_devices!(device_manager, buzzer, imu);
        // TODO move this to main
        unwrap!(imu.reset().await);

        let mut ticker = Ticker::every(device_manager.clock, delay, 5.0);
        let mut calibrator = InteractiveCalibrator::new(Some(0.05), None, None);
        loop {
            ticker.next().await;
            let reading = imu.read().await.map_err(|_| ()).unwrap();
            let next_state = calibrator.process(&reading);
            if let Some(state) = next_state {
                match state {
                    WaitingStill => self.waiting_still_sound(&mut buzzer).await,
                    State(axis, direction, event) if event != Event::Variance => {
                        self.axis_sound(axis, &mut buzzer).await;
                        self.direction_sound(direction, &mut buzzer).await;
                        self.event_sound(event, &mut buzzer).await;
                    }
                    State(_, _, _) => {}
                    Success => {
                        self.success_sound(&mut buzzer).await;
                    }
                    Failure => {
                        self.failure_sound(&mut buzzer).await;
                    }
                    Idle => {}
                }

                debugger.dispatch(DebuggerTargetEvent::Calibrating(state));

                if let Success = state {
                    let cal_info = calibrator.get_calibration_info().unwrap();
                    info!("{}", cal_info);
                    unwrap!(
                        self.vlfs
                            .remove_files_with_type(CALIBRATION_FILE_TYPE)
                            .await
                    );
                    let file = unwrap!(self.vlfs.create_file(CALIBRATION_FILE_TYPE).await);
                    let mut file = unwrap!(self.vlfs.open_file_for_write(file.id).await);

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
    }
}
