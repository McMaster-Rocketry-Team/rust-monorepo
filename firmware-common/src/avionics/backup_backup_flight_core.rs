use super::flight_core_event::FlightCoreState as EventFlightCoreState;
use super::{
    flight_core_event::{FlightCoreEvent, FlightCoreEventPublisher},
    flight_profile::FlightProfile,
};

enum BackupBackupFlightCoreState {
    Armed,
    DrogueChuteDeployed { main_deploy_time: f64 },
    MainChuteDeployed { land_time: f64 },
    Landed,
}

pub struct BackupBackupFlightCore<P: FlightCoreEventPublisher> {
    event_publisher: P,
    flight_profile: FlightProfile,
    state: BackupBackupFlightCoreState,
}

impl<P: FlightCoreEventPublisher> BackupBackupFlightCore<P> {
    pub fn new(flight_profile: FlightProfile, event_publisher: P) -> Self {
        Self {
            event_publisher,
            flight_profile,
            state: BackupBackupFlightCoreState::Armed,
        }
    }

    pub fn manual_deployment_triggered(&mut self, timestamp: f64) {
        if matches!(self.state, BackupBackupFlightCoreState::Armed) {
            self.event_publisher.publish(FlightCoreEvent::ChangeState(
                EventFlightCoreState::DrogueChuteDeployed,
            ));
            self.state = BackupBackupFlightCoreState::DrogueChuteDeployed {
                main_deploy_time: timestamp + self.flight_profile.drouge_to_main_ms,
            };
        }
    }

    pub fn tick(&mut self, timestamp: f64) {
        match &mut self.state {
            BackupBackupFlightCoreState::Armed => {}
            BackupBackupFlightCoreState::DrogueChuteDeployed { main_deploy_time } => {
                if timestamp >= *main_deploy_time {
                    self.event_publisher.publish(FlightCoreEvent::ChangeState(
                        EventFlightCoreState::MainChuteDeployed,
                    ));
                    self.state = BackupBackupFlightCoreState::MainChuteDeployed {
                        land_time: timestamp + self.flight_profile.main_to_landed_ms,
                    };
                }
            }
            BackupBackupFlightCoreState::MainChuteDeployed { land_time } => {
                if timestamp >= *land_time {
                    self.event_publisher
                        .publish(FlightCoreEvent::ChangeState(EventFlightCoreState::Landed));
                    self.state = BackupBackupFlightCoreState::Landed;
                }
            }
            BackupBackupFlightCoreState::Landed => {}
        }
    }
}
