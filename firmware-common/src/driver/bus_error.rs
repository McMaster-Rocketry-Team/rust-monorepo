use super::{i2c::I2CError, spi::SpiBusError};

#[derive(Debug, defmt::Format)]
pub enum BusError {
    SpiBusError(SpiBusError),
    I2CError(I2CError),
    Other,
}

impl From<SpiBusError> for BusError {
    fn from(value: SpiBusError) -> Self {
        Self::SpiBusError(value)
    }
}

impl From<I2CError> for BusError {
    fn from(value: I2CError) -> Self {
        Self::I2CError(value)
    }
}

impl From<()> for BusError {
    fn from(_: ()) -> Self {
        Self::Other
    }
}
