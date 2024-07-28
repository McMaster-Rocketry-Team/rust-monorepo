use heapless::Deque;

use crate::{common::sensor_reading::SensorReading, driver::{barometer::BaroData, timestamp::BootTimestamp}};

pub struct BaroFilterOutput {
    pub should_ignore: bool,
    pub baro_reading: SensorReading<BootTimestamp, BaroData>,
}

#[derive(Clone)]
pub struct BaroReadingFilter {
    history: Deque<SensorReading<BootTimestamp, BaroData>, 2>,
    ignore_pressure_end_time: f64,
    baro_reading_hold: Option<SensorReading<BootTimestamp, BaroData>>,
}

impl BaroReadingFilter {
    pub fn new() -> Self {
        Self {
            history: Deque::new(),
            ignore_pressure_end_time: 0.0,
            baro_reading_hold: None,
        }
    }

    pub fn feed(&mut self, baro_reading: &SensorReading<BootTimestamp, BaroData>) -> BaroFilterOutput {
        if self.history.is_full() {
            self.history.pop_front();
        }
        self.history.push_back(baro_reading.clone()).unwrap();

        if !self.history.is_full() {
            return BaroFilterOutput {
                should_ignore: false,
                baro_reading: baro_reading.clone(),
            };
        }

        let prev_reading = self.history.front().unwrap();
        let pressure_slope = (baro_reading.data.pressure - prev_reading.data.pressure)
            / ((baro_reading.timestamp - prev_reading.timestamp) / 1000.0) as f32;
        if pressure_slope > 3000.0 {
            log_info!("BaroFilter: ignoring pressure reading");
            self.ignore_pressure_end_time = baro_reading.timestamp + 500.0;
            self.baro_reading_hold = Some(baro_reading.clone());
        }

        if baro_reading.timestamp < self.ignore_pressure_end_time {
            return BaroFilterOutput {
                should_ignore: true,
                baro_reading: self.baro_reading_hold.clone().unwrap(),
            };
        } else {
            return BaroFilterOutput {
                should_ignore: false,
                baro_reading: baro_reading.clone(),
            };
        }
    }

    pub fn last_reading(&self) -> Option<SensorReading<BootTimestamp, BaroData>> {
        self.history.back().map(|r| r.clone())
    }
}
