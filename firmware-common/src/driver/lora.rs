#[derive(Copy, Clone)]
pub enum Bandwidth {
    _125KHz,
    _250KHz,
    _500KHz,
}

#[derive(Copy, Clone)]
pub enum SpreadingFactor {
    _7,
    _8,
    _9,
    _10,
    _11,
    _12,
}

#[derive(Copy, Clone)]
pub enum CodingRate {
    _4_5,
    _4_6,
    _4_7,
    _4_8,
}

#[derive(Copy, Clone)]
pub struct RfConfig {
    pub frequency: u32,
    pub bandwidth: Bandwidth,
    pub spreading_factor: SpreadingFactor,
    pub coding_rate: CodingRate,
    pub iq_inverted: bool,
}

#[derive(Copy, Clone)]
pub struct TxConfig {
    pub power: i8,
    pub rf: RfConfig,
}

#[derive(Copy, Clone)]
pub struct RxConfig {
    pub rf: RfConfig,
}

#[derive(Copy, Clone)]
pub struct RxQuality {
    pub rssi: i16,
    pub snr: i8,
}

pub trait LoRa {
    async fn sleep(&mut self) -> Result<(), ()>;
    async fn reset(&mut self) -> Result<(), ()>;
    fn min_power(&self) -> i8;
    fn max_power(&self) -> i8;

    async fn set_rx_config(&mut self, rx_config: RxConfig) -> Result<(), ()>;
    async fn set_tx_config(&mut self, tx_config: TxConfig) -> Result<(), ()>;

    async fn tx(&mut self, data: &[u8]) -> Result<(), ()>;
    async fn rx(&mut self, buffer: &mut [u8], timeout_ms: u32) -> Result<(usize, RxQuality), ()>;
}
