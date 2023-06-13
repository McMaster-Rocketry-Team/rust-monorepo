pub trait RadioApplicationLayer {
    type Error: defmt::Format;

    async fn send(&mut self, package: ApplicationLayerPackage) -> Result<(), Self::Error>;

    async fn receive(&mut self, timeout_ms: f64) -> Result<ApplicationLayerPackage, Self::Error>;
}

#[derive(Debug, Clone, defmt::Format)]
pub enum ApplicationLayerPackage {
    // stand the rocket vertically so VLF can know which angle it is mounted at
    VerticalCalibration,
    SoftArming(bool),
}
