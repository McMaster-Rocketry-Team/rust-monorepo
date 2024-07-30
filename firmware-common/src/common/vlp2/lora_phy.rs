use crate::common::device_config::LoraConfig;
pub use embedded_hal_async::delay::DelayNs;
use heapless::Vec;
use lora_phy::{
    mod_params::{PacketStatus, RadioError},
    mod_traits::RadioKind,
    LoRa, RxMode,
};

pub struct LoraPhy<'a, 'b, LK: RadioKind, DL: DelayNs> {
    lora: &'a mut LoRa<LK, DL>,
    lora_config: &'b LoraConfig,
}

impl<'a, 'b, LK: RadioKind, DL: DelayNs> LoraPhy<'a, 'b, LK, DL> {
    pub fn new(lora: &'a mut LoRa<LK, DL>, lora_config: &'b LoraConfig) -> Self {
        LoraPhy { lora, lora_config }
    }

    pub async fn tx<const N: usize>(&mut self, buffer: &Vec<u8, N>) -> Result<(), RadioError> {
        let modulation_params = self.lora.create_modulation_params(
            self.lora_config.sf_phy(),
            self.lora_config.bw_phy(),
            self.lora_config.cr_phy(),
            self.lora_config.frequencies[0],
        )?;
        let mut tx_params =
            self.lora
                .create_tx_packet_params(8, false, false, false, &modulation_params)?;

        self.lora
            .prepare_for_tx(
                &modulation_params,
                &mut tx_params,
                self.lora_config.power,
                buffer.as_slice(),
            )
            .await?;
        self.lora.tx().await?;
        Ok(())
    }

    pub async fn rx<const N: usize>(
        &mut self,
        listen_mode: RxMode,
        buffer: &mut Vec<u8, N>,
    ) -> Result<PacketStatus, RadioError> {
        let modulation_params = self.lora.create_modulation_params(
            self.lora_config.sf_phy(),
            self.lora_config.bw_phy(),
            self.lora_config.cr_phy(),
            self.lora_config.frequencies[0],
        )?;
        let rx_pkt_params = self.lora.create_rx_packet_params(
            8,
            false,
            buffer.capacity() as u8,
            false,
            false,
            &modulation_params,
        )?;

        self.lora
            .prepare_for_rx(listen_mode, &modulation_params, &rx_pkt_params)
            .await
            .unwrap();
        unsafe {
            buffer.set_len(buffer.capacity());
        }
        let (len, status) = self.lora.rx(&rx_pkt_params, buffer.as_mut_slice()).await?;
        unsafe {
            buffer.set_len(len as usize);
        }
        Ok(status)
    }
}