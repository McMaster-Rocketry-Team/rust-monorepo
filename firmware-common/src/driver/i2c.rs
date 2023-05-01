#[derive(Debug, defmt::Format)]
pub enum I2CError {
    Timeout { ms: u64 },
    Other,
}
