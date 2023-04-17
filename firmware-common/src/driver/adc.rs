pub trait ADC {
    async fn read(&mut self) -> f32; // ma
}
