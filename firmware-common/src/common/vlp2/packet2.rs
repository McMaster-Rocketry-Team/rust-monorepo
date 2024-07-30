use crate::common::delta_logger::prelude::*;

use super::telemetry_packet2::TelemetryPacket;

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
pub enum VLPUplinkPacket {
    VerticalCalibrationPacket(VerticalCalibrationPacket),
    SoftArmPacket(SoftArmPacket),
    LowPowerModePacket(LowPowerModePacket),
    ResetPacket(ResetPacket),
    DeleteLogsPacket(DeleteLogsPacket),
}

#[derive(defmt::Format, Debug, Clone, PartialEq)]
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

#[derive(defmt::Format, Debug, Clone, PartialEq)]
pub enum VLPDownlinkPacket {
    AckPacket(AckPacket),
    TelemetryPacket(TelemetryPacket),
}
