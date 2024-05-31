pub trait TimestampType {}

#[derive(defmt::Format, Debug, Clone)]
pub struct UnixTimestamp;

impl TimestampType for UnixTimestamp {}

#[derive(defmt::Format, Debug, Clone)]
pub struct BootTimestamp;

impl TimestampType for BootTimestamp {}
