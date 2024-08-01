use crate::{avionics::flight_profile::PyroSelection, common::delta_logger::prelude::*};

use super::telemetry_packet::TelemetryPacket;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct VerticalCalibrationPacket {
    pub timestamp: f64,
}

impl BitArraySerializable for VerticalCalibrationPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct SoftArmPacket {
    pub timestamp: f64,
    pub armed: bool,
}

impl BitArraySerializable for SoftArmPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
        writer.write(self.armed);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
            armed: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64 + 1
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct LowPowerModePacket {
    pub timestamp: f64,
    pub enabled: bool,
}

impl BitArraySerializable for LowPowerModePacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
        writer.write(self.enabled);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
            enabled: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64 + 1
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct ResetPacket {
    pub timestamp: f64,
}

impl BitArraySerializable for ResetPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct DeleteLogsPacket {
    pub timestamp: f64,
}

impl BitArraySerializable for DeleteLogsPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct GroundTestDeployPacket {
    pub timestamp: f64,
    pub pyro: PyroSelection,
}

impl BitArraySerializable for GroundTestDeployPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
        writer.write::<u8>(self.pyro.into());
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
            pyro: PyroSelection::try_from(reader.read::<u8>().unwrap()).unwrap(),
        }
    }

    fn len_bits() -> usize {
        64 + 8
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub enum VLPUplinkPacket {
    VerticalCalibrationPacket(VerticalCalibrationPacket),
    SoftArmPacket(SoftArmPacket),
    LowPowerModePacket(LowPowerModePacket),
    ResetPacket(ResetPacket),
    DeleteLogsPacket(DeleteLogsPacket),
    GroundTestDeployPacket(GroundTestDeployPacket),
}

impl From<VerticalCalibrationPacket> for VLPUplinkPacket {
    fn from(packet: VerticalCalibrationPacket) -> Self {
        Self::VerticalCalibrationPacket(packet)
    }
}

impl From<SoftArmPacket> for VLPUplinkPacket {
    fn from(packet: SoftArmPacket) -> Self {
        Self::SoftArmPacket(packet)
    }
}

impl From<LowPowerModePacket> for VLPUplinkPacket {
    fn from(packet: LowPowerModePacket) -> Self {
        Self::LowPowerModePacket(packet)
    }
}

impl From<ResetPacket> for VLPUplinkPacket {
    fn from(packet: ResetPacket) -> Self {
        Self::ResetPacket(packet)
    }
}

impl From<DeleteLogsPacket> for VLPUplinkPacket {
    fn from(packet: DeleteLogsPacket) -> Self {
        Self::DeleteLogsPacket(packet)
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub struct AckPacket {
    pub timestamp: f64,
}

impl BitArraySerializable for AckPacket {
    fn serialize<const N: usize>(&self, writer: &mut BitSliceWriter<N>) {
        writer.write(self.timestamp);
    }

    fn deserialize<const N: usize>(reader: &mut BitSliceReader<N>) -> Self {
        Self {
            timestamp: reader.read().unwrap(),
        }
    }

    fn len_bits() -> usize {
        64
    }
}

#[derive(defmt::Format, Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
pub enum VLPDownlinkPacket {
    AckPacket(AckPacket),
    TelemetryPacket(TelemetryPacket),
}

impl From<AckPacket> for VLPDownlinkPacket {
    fn from(packet: AckPacket) -> Self {
        Self::AckPacket(packet)
    }
}

impl From<TelemetryPacket> for VLPDownlinkPacket {
    fn from(packet: TelemetryPacket) -> Self {
        Self::TelemetryPacket(packet)
    }
}
