pub trait RadioApplicationLayer {
    type Error: defmt::Format;

    async fn send(&mut self, package: ApplicationLayerPackage) -> Result<(), Self::Error>;

    async fn receive(&mut self, timeout_ms: f64) -> Result<ApplicationLayerPackage, Self::Error>;
}

pub enum ApplicationLayerPackage {
    // stand the rocket vertically so VLF can know which angle it is mounted at
    RocketVerticalCalibration,
    SoftArming(bool),
}
