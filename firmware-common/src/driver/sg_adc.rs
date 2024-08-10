use embassy_sync::blocking_mutex::raw::RawMutex;

use crate::{common::zerocopy_channel::ZeroCopyChannel, strain_gauges::SAMPLES_PER_READ};

#[derive(Debug, Clone)]
pub struct RawSGReadings {
    pub start_time: f64, // boot time in ms
    pub sg_readings: [[f32; SAMPLES_PER_READ]; 4],
}

impl RawSGReadings {
    pub const fn new_const() -> Self {
        Self {
            start_time: 0.0,
            sg_readings: [[0.0; SAMPLES_PER_READ]; 4],
        }
    }
}

pub trait SGAdc {
    type Error: defmt::Format + core::fmt::Debug;

    async fn reset(&mut self) -> Result<(), Self::Error>;

    async fn read<M: RawMutex>(
        &mut self,
        channel: &ZeroCopyChannel<M, RawSGReadings>,
    ) -> Result<(), Self::Error>;
}

pub trait SGAdcController {
    async fn set_enable(&mut self, enable: bool);
}
