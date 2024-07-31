use libm::fabsf;

use crate::{
    common::sensor_reading::SensorReading,
    driver::{barometer::BaroData, timestamp::BootTimestamp},
};

use super::{
    flight_core_event::{FlightCoreEvent, FlightCoreEventDispatcher},
    flight_profile::FlightProfile,
    vertical_speed_filter::VerticalSpeedFilter,
};

pub enum BackupFlightCoreState {
    Armed,
    DrogueChute { deploy_time: f64 },
    DrogueChuteDeployed,
    MainChute { deploy_time: f64 },
    MainChuteDeployed,
    Landed,
}

// Designed to run at 200hz
pub struct BackupFlightCore<D: FlightCoreEventDispatcher> {
    event_dispatcher: D,
    flight_profile: FlightProfile,
    state: BackupFlightCoreState,
    vertical_speed_filter: VerticalSpeedFilter,
    launch_pad_altitude: Option<f32>,
}

impl<D: FlightCoreEventDispatcher> BackupFlightCore<D> {
    pub fn new(flight_profile: FlightProfile, event_dispatcher: D) -> Self {
        Self {
            event_dispatcher,
            flight_profile,
            state: BackupFlightCoreState::Armed,
            vertical_speed_filter: VerticalSpeedFilter::new(200.0),
            launch_pad_altitude: None,
        }
    }

    pub fn tick(&mut self, baro_reading: &SensorReading<BootTimestamp, BaroData>) {
        let timestamp = baro_reading.timestamp;
        let vertical_speed = self.vertical_speed_filter.feed(baro_reading);

        match &mut self.state {
            BackupFlightCoreState::Armed => {
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
                    self.event_dispatcher
                        .dispatch(FlightCoreEvent::DeployDrogue)
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
                    self.event_dispatcher.dispatch(FlightCoreEvent::DeployMain)
                }
            }
            BackupFlightCoreState::MainChuteDeployed => {
                if fabsf(vertical_speed) < -0.5 {
                    self.state = BackupFlightCoreState::Landed;
                    self.event_dispatcher.dispatch(FlightCoreEvent::Landed)
                }
            }
            BackupFlightCoreState::Landed => {}
        }
    }
}
