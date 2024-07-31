use crate::{
    common::sensor_reading::SensorReading,
    driver::{barometer::BaroData, timestamp::BootTimestamp},
};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz as _, Type, Q_BUTTERWORTH_F32};
use libm::fabsf;

pub struct VerticalSpeedFilter {
    hold_end_time: f64,
    prev_altitude: Option<(f64, f32)>,
    vertical_speed_filter: DirectForm2Transposed<f32>,
    last_vertical_speed: Option<f32>,
}

impl VerticalSpeedFilter {
    pub fn new(sampling_freq: f32) -> Self {
        let cut_off_freq = (0.2).hz();

        let coeffs = Coefficients::<f32>::from_params(
            Type::LowPass,
            sampling_freq.hz(),
            cut_off_freq,
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        Self {
            hold_end_time: 0.0,
            prev_altitude: None,
            vertical_speed_filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_vertical_speed: None,
        }
    }

    /// returns vertical speed
    pub fn feed(&mut self, baro_reading: &SensorReading<BootTimestamp, BaroData>) -> f32 {
        if let Some(vertical_speed) = self.feed_inner(baro_reading) {
            self.last_vertical_speed = Some(vertical_speed);
            return vertical_speed;
        } else {
            return self.last_vertical_speed.unwrap_or(0.0);
        }
    }

    /// returns vertical speed
    fn feed_inner(&mut self, baro_reading: &SensorReading<BootTimestamp, BaroData>) -> Option<f32> {
        if baro_reading.timestamp < self.hold_end_time {
            return None;
        }

        let altitude = baro_reading.data.altitude();

        if let Some((prev_timestamp, prev_altitude)) = self.prev_altitude {
            let vertical_speed = (altitude - prev_altitude)
                / ((baro_reading.timestamp - prev_timestamp) as f32 / 1000.0);

            if fabsf(vertical_speed) > 3000.0 {
                self.hold_end_time = baro_reading.timestamp + 1000.0;
                self.prev_altitude = None;

                return None;
            } else {
                let filtered_vertical_speed = self.vertical_speed_filter.run(vertical_speed);
                return Some(filtered_vertical_speed);
            }
        } else {
            self.prev_altitude = Some((baro_reading.timestamp, altitude));
            return None;
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_biquad() {
        use biquad::*;

        // Cutoff and sampling frequencies
        let f0 = (0.2).hz();
        let fs = 200.hz();

        // Create coefficients for the biquads
        let coeffs =
            Coefficients::<f32>::from_params(Type::LowPass, fs, f0, Q_BUTTERWORTH_F32).unwrap();

        // Create two different biquads
        let mut biquad1 = DirectForm2Transposed::<f32>::new(coeffs);

        let input_vec = vec![10.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let mut output_vec1 = Vec::new();

        // Run for all the inputs
        for elem in input_vec {
            output_vec1.push(biquad1.run(elem));
        }

        println!("{:?}", output_vec1);
    }
}
