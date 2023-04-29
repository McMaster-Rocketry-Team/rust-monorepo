use super::spi::SpiBusError;

#[derive(Debug, defmt::Format)]
pub enum BusError {
    SpiBusError(SpiBusError),
}

impl From<SpiBusError> for BusError{
    fn from(value: SpiBusError) -> Self {
        Self::SpiBusError(value)
    }
}