use embassy_sync::blocking_mutex::raw::RawMutex;

use crate::common::zerocopy_channel::ZeroCopyChannelSender;

pub trait RawSGReadingsTrait: Default {
    type Iter<'a>: Iterator<Item = f32>
    where
        Self: 'a;

    fn get_start_time(&self) -> f64;

    /// needs to at least return crate::strain_gauges::SAMPLES_PER_READ number of readings
    fn get_sg_readings_iter(&self, sg_i: usize) -> Self::Iter<'_>;
}

pub trait SGAdc<T: RawSGReadingsTrait> {
    type Error: defmt::Format + core::fmt::Debug;

    async fn read<'a, M: RawMutex, const N: usize>(
        &mut self,
        sender: &mut ZeroCopyChannelSender<'a, M, T, N>,
    ) -> Result<(), Self::Error>;
}

pub trait SGAdcController {
    async fn set_enable(&mut self, enable: bool);
}
