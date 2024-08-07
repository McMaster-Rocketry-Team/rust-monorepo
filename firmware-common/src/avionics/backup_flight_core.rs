use libm::fabsf;

use super::flight_core_event::FlightCoreState as EventFlightCoreState;
use crate::{
    common::sensor_reading::SensorReading,
    driver::{barometer::BaroData, timestamp::BootTimestamp},
};

use super::{
    flight_core_event::{FlightCoreEvent, FlightCoreEventPublisher},
    flight_profile::FlightProfile,
    vertical_speed_filter::VerticalSpeedFilter,
};

enum BackupFlightCoreState {
    Armed,
    DrogueChute { deploy_time: f64 },
    DrogueChuteDeployed,
    MainChute { deploy_time: f64 },
    MainChuteDeployed,
    Landed,
}

// Designed to run at 200hz
pub struct BackupFlightCore<P: FlightCoreEventPublisher> {
    event_publisher: P,
    flight_profile: FlightProfile,
    state: BackupFlightCoreState,
    vertical_speed_filter: VerticalSpeedFilter,
    launch_pad_altitude: Option<f32>,
    first_tick: bool,
}

impl<D: FlightCoreEventPublisher> BackupFlightCore<D> {
    pub fn new(flight_profile: FlightProfile, event_publisher: D) -> Self {
        Self {
            event_publisher,
            flight_profile,
            state: BackupFlightCoreState::Armed,
            vertical_speed_filter: VerticalSpeedFilter::new(200.0),
            launch_pad_altitude: None,
            first_tick: true,
        }
    }

    pub fn tick(&mut self, baro_reading: &SensorReading<BootTimestamp, BaroData>) {
        let timestamp = baro_reading.timestamp;
        let vertical_speed = self.vertical_speed_filter.feed(baro_reading);

        match &mut self.state {
            BackupFlightCoreState::Armed => {
                if self.first_tick {
                    self.event_publisher
                        .publish(FlightCoreEvent::ChangeState(EventFlightCoreState::Armed));
                    self.first_tick = false;
                }

                if self.launch_pad_altitude.is_none() {
                    self.launch_pad_altitude = Some(baro_reading.data.altitude());
                }
                if vertical_speed < -10.0 {
                    self.state = BackupFlightCoreState::DrogueChute {
                        deploy_time: timestamp + self.flight_profile.drogue_chute_delay_ms,
                    };
                }
            }
            BackupFlightCoreState::DrogueChute { deploy_time } => {
                if timestamp > *deploy_time {
                    self.state = BackupFlightCoreState::DrogueChuteDeployed;
                    self.event_publisher.publish(FlightCoreEvent::ChangeState(
                        EventFlightCoreState::DrogueChuteDeployed,
                    ));
                }
            }
            BackupFlightCoreState::DrogueChuteDeployed => {
                if baro_reading.data.altitude() < self.flight_profile.main_chute_altitude_agl {
                    self.state = BackupFlightCoreState::MainChute {
                        deploy_time: timestamp + self.flight_profile.main_chute_delay_ms,
                    };
                }
            }
            BackupFlightCoreState::MainChute { deploy_time } => {
                if timestamp > *deploy_time {
                    self.state = BackupFlightCoreState::MainChuteDeployed;
                    self.event_publisher.publish(FlightCoreEvent::ChangeState(
                        EventFlightCoreState::MainChuteDeployed,
                    ));
                }
            }
            BackupFlightCoreState::MainChuteDeployed => {
                if fabsf(vertical_speed) < -0.5 {
                    self.state = BackupFlightCoreState::Landed;
                    self.event_publisher
                        .publish(FlightCoreEvent::ChangeState(EventFlightCoreState::Landed));
                }
            }
            BackupFlightCoreState::Landed => {}
        }
    }
}

impl<D: FlightCoreEventPublisher> Drop for BackupFlightCore<D> {
    fn drop(&mut self) {
        self.event_publisher
            .publish(FlightCoreEvent::ChangeState(EventFlightCoreState::DisArmed));
    }
}

#[cfg(test)]
mod test {
    use icao_isa::calculate_isa_pressure;
    use icao_units::si::{Metres, Pascals};

    use crate::{
        common::sensor_reading::SensorReading,
        driver::{barometer::BaroData, timestamp::BootTimestamp},
    };

    #[test]
    fn test_flight_core() {
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
                let time = start_time + i as f64 * 5.0;
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
    }
}
