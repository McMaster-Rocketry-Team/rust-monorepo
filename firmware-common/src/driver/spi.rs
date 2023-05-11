#[derive(Debug, defmt::Format)]
pub enum SpiBusError {
    Timeout { ms: u64 },
    Other,
}
