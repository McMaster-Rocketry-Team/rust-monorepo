use rkyv::{Archive, Deserialize, Serialize};

use crate::create_serialized_enum;

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct VerticalCalibrationPackage {
    timestamp: f64,
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct SoftArmPackage {
    timestamp: f64,
    armed: bool,
}

create_serialized_enum!(
    VLPUplinkPacketWriter,
    VLPUplinkPacketReader,
    VLPUplinkPacket,
    (0, VerticalCalibrationPackage),
    (1, SoftArmPackage)
);

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct Ack {
    timestamp: f64,
}

create_serialized_enum!(
    VLPDownlinkPacketWriter,
    VLPDownlinkPacketReader,
    VLPDownlinkPacket,
    (0, Ack)
);
