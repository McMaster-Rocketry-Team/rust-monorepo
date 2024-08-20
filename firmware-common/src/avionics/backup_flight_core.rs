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
        self.event_publisher
            .publish(FlightCoreEvent::ChangeAirSpeed(vertical_speed));

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
                if vertical_speed < -20.0 {
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
                if baro_reading.data.altitude() < self.flight_profile.main_chute_altitude_agl + self.launch_pad_altitude.unwrap() {
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
                if fabsf(vertical_speed) < 1.0 {
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
    use super::*;
    use crate::{
        avionics::flight_profile::{FlightProfile, PyroSelection},
        common::sensor_reading::SensorReading,
        driver::{barometer::BaroData, timestamp::BootTimestamp},
    };
    use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};

    #[test]
    fn test_flight_core_106() {
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

        let flight_profile = FlightProfile {
            drogue_pyro: PyroSelection::Pyro1,
            drogue_chute_minimum_time_ms: 10000.0,
            drogue_chute_minimum_altitude_agl: 1500.0,
            drogue_chute_delay_ms: 1000.0,
            main_pyro: PyroSelection::Pyro2,
            main_chute_altitude_agl: 500.0,
            main_chute_delay_ms: 1000.0,
            drouge_to_main_ms: 107000.0,
            main_to_landed_ms: 76000.0,
        };
        let channel = Channel::<NoopRawMutex, FlightCoreEvent, 10>::new();
        let receiver = channel.receiver();

        let mut flight_core = BackupFlightCore::new(flight_profile, channel.sender());
        for reading in &baro_readings {
            flight_core.tick(reading);
            while let Ok(event) = receiver.try_receive() {
                match event {
                    FlightCoreEvent::CriticalError => todo!(),
                    FlightCoreEvent::DidNotReachMinApogee => todo!(),
                    FlightCoreEvent::ChangeState(new_state) => {
                        println!("{}: State changed to {:?}", reading.timestamp, new_state);
                    }
                    FlightCoreEvent::ChangeAltitude(_) => {}
                    FlightCoreEvent::ChangeAirSpeed(_) => {}
                }
            }
        }
    }
}
