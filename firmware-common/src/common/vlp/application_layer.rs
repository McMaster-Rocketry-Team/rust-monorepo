use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};

pub trait RadioApplicationClient {
    type Error: defmt::Format;

    async fn run<R: RawMutex, const N: usize, const M: usize>(
        &mut self,
        radio_tx: Receiver<R, ApplicationLayerTxPackage, N>,
        radio_rx: Sender<R, ApplicationLayerRxPackage, M>,
    ) -> !;
}

#[derive(Debug, Clone, defmt::Format)]
pub enum ApplicationLayerRxPackage {
    // stand the rocket vertically so VLF can know which angle it is mounted at
    VerticalCalibration,
    // currently unused
    SoftArming(bool),
}

#[derive(Debug, Clone, defmt::Format)]
pub enum ApplicationLayerTxPackage {
    Telemetry,
}
