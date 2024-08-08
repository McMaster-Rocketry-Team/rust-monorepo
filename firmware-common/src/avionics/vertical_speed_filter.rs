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
                self.hold_end_time = baro_reading.timestamp + 500.0;
                self.prev_altitude = None;

                return None;
            } else {
                let filtered_vertical_speed = self.vertical_speed_filter.run(vertical_speed);
                self.prev_altitude = Some((baro_reading.timestamp, altitude));
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
    use crate::{
        common::sensor_reading::SensorReading,
        driver::{barometer::BaroData, timestamp::BootTimestamp},
    };
    use icao_isa::calculate_isa_pressure;
    use icao_units::si::{Metres, Pascals};

    #[test]
    fn test_vertical_speed_filtering() {
        let mut baro_readings: Vec<SensorReading<BootTimestamp, BaroData>> =
            vec![SensorReading::new(
                0.0,
                BaroData {
                    temperature: 25.0,
                    pressure: calculate_isa_pressure(Metres(0.0)).0 as f32,
                },
            )];

        let mut lerp = |duration_ms: f64, final_pressure: Pascals| {
            let sample_count = (duration_ms / 5.0) as usize;
            let start_time = baro_readings.last().unwrap().timestamp;
            let start_pressure = baro_readings.last().unwrap().data.pressure;
            let final_pressure = final_pressure.0 as f32;
            for i in 0..sample_count {
                let time = start_time + (i + 1) as f64 * 5.0;
                let pressure = start_pressure
                    + (final_pressure - start_pressure) * (i as f32 / sample_count as f32);
                baro_readings.push(SensorReading::new(
                    time,
                    BaroData {
                        temperature: 25.0,
                        pressure,
                    },
                ));
            }
        };

        lerp(1000.0, calculate_isa_pressure(Metres(0.0)));
        lerp(15000.0, calculate_isa_pressure(Metres(2000.0)));
        lerp(
            10.0,
            Pascals(calculate_isa_pressure(Metres(2000.0)).0 * 2.0),
        );
        lerp(
            500.0,
            Pascals(calculate_isa_pressure(Metres(2000.0)).0 * 1.2),
        );
        lerp(5000.0, calculate_isa_pressure(Metres(3000.0)));
        lerp(30000.0, calculate_isa_pressure(Metres(0.0)));

        println!("readings length: {:?}", baro_readings.len());

        let mut unfiltered_vertical_speed = vec![0f32];
        let mut last_altitude = baro_readings.last().unwrap().data.altitude();
        let mut last_timestamp = baro_readings.last().unwrap().timestamp;
        for reading in baro_readings.iter().skip(1) {
            let altitude = reading.data.altitude();
            let vertical_speed =
                (altitude - last_altitude) / ((reading.timestamp - last_timestamp) as f32 / 1000.0);
            last_altitude = altitude;
            last_timestamp = reading.timestamp;
            unfiltered_vertical_speed.push(vertical_speed);
        }

        let mut filter = super::VerticalSpeedFilter::new(200.0);
        let filtered_vertical_speed = baro_readings
            .iter()
            .map(|reading| filter.feed(reading))
            .collect::<Vec<f32>>();

        use plotters::prelude::*;
        let root_area =
            BitMapBackend::new("test_vertical_speed_filtering.png", (600, 400)).into_drawing_area();
        root_area.fill(&WHITE).unwrap();

        let mut ctx = ChartBuilder::on(&root_area)
            .set_label_area_size(LabelAreaPosition::Left, 40)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_cartesian_2d(
                0f32..(baro_readings.len() as f32 / 200.0),
                -4000f32..4000f32,
            )
            .unwrap();

        ctx.configure_mesh().draw().unwrap();

        ctx.draw_series(LineSeries::new(
            unfiltered_vertical_speed
                .into_iter()
                .enumerate()
                .map(|(i, y)| (i as f32 / 200.0, y)),
            &GREEN,
        ))
        .unwrap();
        ctx.draw_series(LineSeries::new(
            filtered_vertical_speed
                .into_iter()
                .enumerate()
                .map(|(i, y)| (i as f32 / 200.0, y)),
            &RED,
        ))
        .unwrap();
    }

    #[test]
    fn test_vertical_speed_filtering_106() {
        let mut baro_readings: Vec<SensorReading<BootTimestamp, BaroData>> = vec![];

        let mut reader = csv::Reader::from_path("./test-data/106.baro_tier_1.csv").unwrap();
        for result in reader.records().skip(1) {
            let record = result.unwrap();
            let timestamp = record[0].parse::<f64>().unwrap();
            let pressure = record[2].parse::<f32>().unwrap();
            let temperature = record[4].parse::<f32>().unwrap();

            baro_readings.push(SensorReading::new(
                timestamp,
                BaroData {
                    temperature,
                    pressure,
                },
            ));
        }

        println!("readings length: {:?}", baro_readings.len());

        let mut unfiltered_vertical_speed = vec![0f32];
        let mut last_altitude = baro_readings.last().unwrap().data.altitude();
        let mut last_timestamp = baro_readings.last().unwrap().timestamp;
        for reading in baro_readings.iter().skip(1) {
            let altitude = reading.data.altitude();
            let vertical_speed =
                (altitude - last_altitude) / ((reading.timestamp - last_timestamp) as f32 / 1000.0);
            last_altitude = altitude;
            last_timestamp = reading.timestamp;
            unfiltered_vertical_speed.push(vertical_speed);
        }
        println!(
            "unfiltered vertical speed max: {}",
            unfiltered_vertical_speed
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max)
        );

        let mut filter = super::VerticalSpeedFilter::new(200.0);
        let filtered_vertical_speed = baro_readings
            .iter()
            .map(|reading| filter.feed(reading))
            .collect::<Vec<f32>>();

        use plotters::prelude::*;
        let root_area = BitMapBackend::new("test_vertical_speed_filtering.png", (1920, 1080))
            .into_drawing_area();
        root_area.fill(&WHITE).unwrap();

        let mut ctx = ChartBuilder::on(&root_area)
            .set_label_area_size(LabelAreaPosition::Left, 40)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_cartesian_2d(
                0f32..(baro_readings.len() as f32 / 200.0),
                -1000f32..1000f32,
            )
            .unwrap();

        ctx.configure_mesh().draw().unwrap();

        ctx.draw_series(LineSeries::new(
            unfiltered_vertical_speed
                .into_iter()
                .enumerate()
                .map(|(i, y)| (i as f32 / 200.0, y)),
            &GREEN,
        ))
        .unwrap();
        ctx.draw_series(LineSeries::new(
            filtered_vertical_speed
                .into_iter()
                .enumerate()
                .map(|(i, y)| (i as f32 / 200.0, y)),
            &RED,
        ))
        .unwrap();
    }
}
