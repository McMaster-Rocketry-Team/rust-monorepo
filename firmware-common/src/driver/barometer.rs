use super::bus_error::BusError;

#[derive(defmt::Format, Debug)]
pub struct BaroReading {
    pub timestamp: u64,   // ms
    pub temperature: f32, // C
    pub pressure: f32,    // Pa
}

pub trait Barometer {
    async fn reset(&mut self) -> Result<(), BusError>;
    async fn read(&mut self) -> Result<BaroReading, BusError>;
}
