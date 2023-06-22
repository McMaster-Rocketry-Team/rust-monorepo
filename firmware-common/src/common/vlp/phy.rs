use core::ops::DerefMut;
use serde::Serialize as SerdeSerialize;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
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

#[derive(SerdeSerialize, Debug, defmt::Format)]
pub struct RadioReceiveInfo {
    pub rssi: i16,
    pub snr: i16,
    pub len: u8,
}

pub trait VLPPhy {
    async fn tx(&mut self, payload: &[u8]);
    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError>;
    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError>;

    fn set_frequency(&mut self, frequency: u32);
    fn increment_frequency(&mut self);
    fn reset_frequency(&mut self);
    fn set_output_power(&mut self, power: i32);
}

impl<'a, M, T> VLPPhy for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: VLPPhy,
{
    async fn tx(&mut self, payload: &[u8]) {
        self.deref_mut().tx(payload).await
    }

    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
        self.deref_mut().rx().await
    }

    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
        self.deref_mut().rx_with_timeout(timeout_ms).await
    }

    fn increment_frequency(&mut self) {
        self.deref_mut().increment_frequency()
    }

    fn set_frequency(&mut self, frequency: u32) {
        self.deref_mut().set_frequency(frequency)
    }

    fn reset_frequency(&mut self) {
        self.deref_mut().reset_frequency()
    }

    fn set_output_power(&mut self, power: i32) {
        self.deref_mut().set_output_power(power)
    }
}

pub struct PhysicalVLPPhy<'a, R: RadioKind + 'static, T: Timer> {
    phy: &'a mut LoRa<R>,
    tim: T,
    freq_idx: usize,
    power: i32,
}

impl<'a, R: RadioKind + 'static, T: Timer> PhysicalVLPPhy<'a, R, T> {
    // power: -9 - 22
    pub fn new(phy: &'a mut LoRa<R>, tim: T) -> Self {
        Self {
            phy,
            tim,
            power: -9,
            freq_idx: 0,
        }
    }

    fn create_modulation_params(&mut self) -> ModulationParams {
        let freq = FREQ_LIST[self.freq_idx];

        self.phy
            .create_modulation_params(
                SpreadingFactor::_12,
                Bandwidth::_500KHz,
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
            .prepare_for_tx(&modulation_params, self.power, true)
            .await
            .unwrap();
        self.phy
            .tx(&modulation_params, &mut tx_params, payload, 0x00FFFFFF)
            .await
            .unwrap();
        self.phy
            .sleep(&mut DelayUsWrapper(self.tim))
            .await
            .expect("Sleep failed");
    }

    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
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
        let result = match self.phy.rx(&rx_params, &mut buf[..]).await {
            Ok((bytes, status)) => {
                let info = RadioReceiveInfo {
                    rssi: status.rssi,
                    snr: status.snr,
                    len: bytes,
                };
                Ok((
                    info,
                    Vec::<u8, MAX_PAYLOAD_LENGTH>::from_slice(&buf[..(bytes as usize)]).unwrap(),
                ))
            }
            Err(e) => Err(e),
        };
        self.phy
            .sleep(&mut DelayUsWrapper(self.tim))
            .await
            .expect("Sleep failed");
        result
    }

    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
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

        let result = match fut {
            Ok(Ok((bytes, status))) => {
                let info = RadioReceiveInfo {
                    rssi: status.rssi,
                    snr: status.snr,
                    len: bytes,
                };
                Ok((info, Vec::<u8, MAX_PAYLOAD_LENGTH>::from_slice(&buf[..(bytes as usize)]).unwrap()))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RadioError::ReceiveTimeout),
        };
        self.phy
            .sleep(&mut DelayUsWrapper(self.tim))
            .await
            .expect("Sleep failed");
        result
    }

    fn set_frequency(&mut self, frequency: u32) {
        let mut idx = 0;
        for (i, freq) in FREQ_LIST.iter().enumerate() {
            if *freq == frequency {
                idx = i;
                break;
            }
        }
        self.freq_idx = idx;
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

    fn set_output_power(&mut self, power: i32) {
        self.power = power;
    }
}
