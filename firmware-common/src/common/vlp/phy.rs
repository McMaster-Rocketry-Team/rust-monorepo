use heapless::Vec;
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, ModulationParams, RadioError, SpreadingFactor},
    mod_traits::RadioKind,
    LoRa,
};

use crate::{
    driver::timer::{DelayUsWrapper, Timer},
    utils::run_with_timeout,
};

use super::MAX_PAYLOAD_LENGTH;

const FREQ_LIST: [u32; 64] = [
    902300000, 902500000, 902700000, 902900000, 903100000, 903300000, 903500000, 903700000,
    903900000, 904100000, 904300000, 904500000, 904700000, 904900000, 905100000, 905300000,
    905500000, 905700000, 905900000, 906100000, 906300000, 906500000, 906700000, 906900000,
    907100000, 907300000, 907500000, 907700000, 907900000, 908100000, 908300000, 908500000,
    908700000, 908900000, 909100000, 909300000, 909500000, 909700000, 909900000, 910100000,
    910300000, 910500000, 910700000, 910900000, 911100000, 911300000, 911500000, 911700000,
    911900000, 912100000, 912300000, 912500000, 912700000, 912900000, 913100000, 913300000,
    913500000, 913700000, 913900000, 914100000, 914300000, 914500000, 914700000, 914900000,
];

pub trait VLPPhy {
    async fn tx(&mut self, payload: &[u8]);
    async fn rx(&mut self) -> Result<Vec<u8, MAX_PAYLOAD_LENGTH>, RadioError>;
    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<Vec<u8, MAX_PAYLOAD_LENGTH>, RadioError>;

    fn increment_frequency(&mut self);
    fn reset_frequency(&mut self);
}

pub struct PhysicalVLPPhy<'a, R: RadioKind + 'static, T: Timer> {
    phy: &'a mut LoRa<R>,
    tim: T,
    freq_idx: usize,
}

impl<'a, R: RadioKind + 'static, T: Timer> PhysicalVLPPhy<'a, R, T> {
    pub fn new(phy: &'a mut LoRa<R>, tim: T) -> Self {
        Self {
            phy,
            tim,
            freq_idx: 0,
        }
    }

    fn create_modulation_params(&mut self) -> ModulationParams {
        let freq = FREQ_LIST[self.freq_idx];

        self.phy
            .create_modulation_params(
                SpreadingFactor::_12,
                Bandwidth::_250KHz,
                CodingRate::_4_8,
                freq,
            )
            .unwrap()
    }
}

impl<'a, R: RadioKind + 'static, T: Timer> VLPPhy for PhysicalVLPPhy<'a, R, T> {
    async fn tx(&mut self, payload: &[u8]) {
        let modulation_params = self.create_modulation_params();
        let mut tx_params = self
            .phy
            .create_tx_packet_params(8, false, true, false, &modulation_params)
            .unwrap();
        self.phy
            .sleep(&mut DelayUsWrapper(self.tim))
            .await
            .expect("Sleep failed");
        self.phy
            .prepare_for_tx(&modulation_params, -9, true)
            .await
            .unwrap();
        self.phy
            .tx(&modulation_params, &mut tx_params, payload, 0x00FFFFFF)
            .await
            .unwrap();
    }

    async fn rx(&mut self) -> Result<Vec<u8, MAX_PAYLOAD_LENGTH>, RadioError> {
        let modulation_params = self.create_modulation_params();
        let rx_params =
            self.phy
                .create_rx_packet_params(8, false, 255, true, false, &modulation_params)?;

        let mut buf = [0; 222];
        self.phy
            .prepare_for_rx(
                &modulation_params,
                &rx_params,
                None,
                true,
                true,
                4,
                0x00FFFFFF,
            )
            .await?;
        match self.phy.rx(&rx_params, &mut buf[..]).await {
            Ok((bytes, _)) => {
                Ok(Vec::<u8, MAX_PAYLOAD_LENGTH>::from_slice(&buf[..(bytes as usize)]).unwrap())
            }
            Err(e) => Err(e),
        }
    }

    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<Vec<u8, MAX_PAYLOAD_LENGTH>, RadioError> {
        let modulation_params = self.create_modulation_params();

        let rx_params = self
            .phy
            .create_rx_packet_params(8, false, 255, true, false, &modulation_params)
            .unwrap();

        self.phy.sleep(&mut DelayUsWrapper(self.tim)).await.unwrap();

        let mut buf = [0; 222];

        self.phy
            .prepare_for_rx(
                &modulation_params,
                &rx_params,
                None,
                true,
                true,
                4,
                0x00FFFFFF,
            )
            .await?;

        let fut = run_with_timeout(
            self.tim,
            timeout_ms as f64,
            self.phy.rx(&rx_params, &mut buf[..]),
        )
        .await;

        match fut {
            Ok(Ok((bytes, _))) => {
                Ok(Vec::<u8, MAX_PAYLOAD_LENGTH>::from_slice(&buf[..(bytes as usize)]).unwrap())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RadioError::ReceiveTimeout),
        }
    }

    fn increment_frequency(&mut self) {
        self.freq_idx += 1;
        if self.freq_idx >= 64 {
            self.freq_idx -= 64;
        }
    }

    fn reset_frequency(&mut self) {
        self.freq_idx = 0;
    }
}
