use heapless::Vec;
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, ModulationParams, RadioError, SpreadingFactor},
    mod_traits::RadioKind,
    LoRa,
};

pub struct VLPPhy<R: RadioKind + 'static> {
    phy: LoRa<R>,
}

impl<R: RadioKind + 'static> VLPPhy<R> {
    pub fn new(phy: LoRa<R>) -> VLPPhy<R> {
        VLPPhy { phy }
    }

    pub async fn tx(&mut self, payload: &[u8]) {
        let modulation_params = self.create_modulation_params();
        let mut tx_params = self
            .phy
            .create_tx_packet_params(8, false, true, false, &modulation_params)
            .unwrap();
        self.phy
            .prepare_for_tx(&modulation_params, 22, true)
            .await
            .unwrap();
        self.phy
            .tx(&modulation_params, &mut tx_params, payload, 0xFFFFFFFF)
            .await
            .unwrap();
    }

    pub async fn rx(&mut self) -> Result<Vec<u8, 256>, RadioError> {
        let modulation_params = self.create_modulation_params();
        let rx_params =
            self.phy
                .create_rx_packet_params(8, false, 255, true, false, &modulation_params)?;

        let mut buf = Vec::<u8, 256>::new();
        self.phy
            .prepare_for_rx(
                &modulation_params,
                &rx_params,
                None,
                true,
                true,
                4,
                0xFFFFFFFF,
            )
            .await?;
        match self.phy.rx(&rx_params, &mut buf[..]).await {
            Ok(_) => Ok(buf),
            Err(e) => Err(e),
        }
    }

    pub async fn rx_with_timeout(&mut self, timeout_ms: u32) -> Result<Vec<u8, 256>, RadioError> {
        let modulation_params = self.create_modulation_params();

        let rx_params = self
            .phy
            .create_rx_packet_params(8, false, 255, true, false, &modulation_params)
            .unwrap();

        let mut buf = Vec::<u8, 256>::new();
        self.phy
            .prepare_for_rx(
                &modulation_params,
                &rx_params,
                None,
                false,
                true,
                4,
                timeout_ms,
            )
            .await
            .unwrap();

        match self.phy.rx(&rx_params, &mut buf[..]).await {
            Ok(_) => Ok(buf),
            Err(e) => Err(e),
        }
    }

    fn create_modulation_params(&mut self) -> ModulationParams {
        self.phy
            .create_modulation_params(
                SpreadingFactor::_12,
                Bandwidth::_250KHz,
                CodingRate::_4_8,
                915_000_000,
            )
            .unwrap()
    }
}
