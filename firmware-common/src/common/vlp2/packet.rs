use crate::common::vlp2::telemetry_packet::{
    ArchivedInFlightDiagnosticTelemetryPacket, ArchivedInFlightNominalTelemetryPacket,
    ArchivedPadDiagnosticTelemetryPacket, ArchivedPadIdleTelemetryPacket,
    InFlightDiagnosticTelemetryPacket, InFlightNominalTelemetryPacket,
    PadDiagnosticTelemetryPacket, PadIdleTelemetryPacket,
};
use core::mem::size_of;
use rkyv::{Archive, Deserialize, Serialize};

use crate::create_serialized_enum;

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct VerticalCalibrationPacket {
    pub timestamp: f64,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct SoftArmPacket {
    pub timestamp: f64,
    pub armed: bool,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct LowPowerModePacket {
    pub timestamp: f64,
    pub enabled: bool,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ResetPacket;

create_serialized_enum!(
    VLPUplinkPacketWriter,
    VLPUplinkPacketReader,
    VLPUplinkPacket,
    (0, VerticalCalibrationPacket),
    (1, SoftArmPacket),
    (2, LowPowerModePacket),
    (3, ResetPacket)
);

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct AckPacket {
    pub timestamp: f64,
}

create_serialized_enum!(
    VLPDownlinkPacketWriter,
    VLPDownlinkPacketReader,
    VLPDownlinkPacket,
    (0, AckPacket),
    (1, PadIdleTelemetryPacket),
    (2, PadDiagnosticTelemetryPacket),
    (3, InFlightNominalTelemetryPacket),
    (4, InFlightDiagnosticTelemetryPacket)
);

const fn max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}

pub const MAX_VLP_UPLINK_PACKET_SIZE: usize =
    size_of::<<VLPUplinkPacket as Archive>::Archived>() + 1;
pub const MAX_VLP_DOWNLINK_PACKET_SIZE: usize =
    size_of::<<VLPDownlinkPacket as Archive>::Archived>() + 1;
pub const MAX_VLP_PACKET_SIZE: usize =
    max(MAX_VLP_UPLINK_PACKET_SIZE, MAX_VLP_DOWNLINK_PACKET_SIZE);
