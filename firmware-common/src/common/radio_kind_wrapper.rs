use core::ops::{Deref, DerefMut};

use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use embedded_hal_async::delay::DelayUs;
use lora_phy::{
    mod_params::{
        BoardType, DutyCycleParams, ModulationParams, PacketParams, PacketStatus, RadioError,
        RadioMode,
    },
    mod_traits::RadioKind,
};

pub struct RadioKindWrapper<'a, M: RawMutex, T: RadioKind>(pub MutexGuard<'a, M, T>);

impl<'a, M, T> RadioKind for RadioKindWrapper<'a, M, T>
where
    M: RawMutex,
    T: RadioKind,
{
    fn get_board_type(&self) -> BoardType {
        self.0.deref().get_board_type()
    }

    async fn reset(&mut self, delay: &mut impl DelayUs) -> Result<(), RadioError> {
        self.0.deref_mut().reset(delay).await
    }

    async fn ensure_ready(&mut self, mode: RadioMode) -> Result<(), RadioError> {
        self.0.deref_mut().ensure_ready(mode).await
    }

    async fn init_rf_switch(&mut self) -> Result<(), RadioError> {
        self.0.deref_mut().init_rf_switch().await
    }

    async fn set_standby(&mut self) -> Result<(), RadioError> {
        self.0.deref_mut().set_standby().await
    }

    async fn set_sleep(&mut self, delay: &mut impl DelayUs) -> Result<bool, RadioError> {
        self.0.deref_mut().set_sleep(delay).await
    }

    async fn set_lora_modem(&mut self, enable_public_network: bool) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .set_lora_modem(enable_public_network)
            .await
    }

    async fn set_oscillator(&mut self) -> Result<(), RadioError> {
        self.0.deref_mut().set_oscillator().await
    }

    async fn set_regulator_mode(&mut self) -> Result<(), RadioError> {
        self.0.deref_mut().set_regulator_mode().await
    }

    async fn set_tx_rx_buffer_base_address(
        &mut self,
        tx_base_addr: usize,
        rx_base_addr: usize,
    ) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .set_tx_rx_buffer_base_address(tx_base_addr, rx_base_addr)
            .await
    }

    async fn set_tx_power_and_ramp_time(
        &mut self,
        output_power: i32,
        mdltn_params: Option<&ModulationParams>,
        tx_boosted_if_possible: bool,
        is_tx_prep: bool,
    ) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .set_tx_power_and_ramp_time(
                output_power,
                mdltn_params,
                tx_boosted_if_possible,
                is_tx_prep,
            )
            .await
    }

    async fn update_retention_list(&mut self) -> Result<(), RadioError> {
        self.0.deref_mut().update_retention_list().await
    }

    async fn set_modulation_params(
        &mut self,
        mdltn_params: &ModulationParams,
    ) -> Result<(), RadioError> {
        self.0.deref_mut().set_modulation_params(mdltn_params).await
    }

    async fn set_packet_params(&mut self, pkt_params: &PacketParams) -> Result<(), RadioError> {
        self.0.deref_mut().set_packet_params(pkt_params).await
    }

    async fn calibrate_image(&mut self, frequency_in_hz: u32) -> Result<(), RadioError> {
        self.0.deref_mut().calibrate_image(frequency_in_hz).await
    }

    async fn set_channel(&mut self, frequency_in_hz: u32) -> Result<(), RadioError> {
        self.0.deref_mut().set_channel(frequency_in_hz).await
    }

    async fn set_payload(&mut self, payload: &[u8]) -> Result<(), RadioError> {
        self.0.deref_mut().set_payload(payload).await
    }

    async fn do_tx(&mut self, timeout_in_ms: u32) -> Result<(), RadioError> {
        self.0.deref_mut().do_tx(timeout_in_ms).await
    }

    async fn do_rx(
        &mut self,
        rx_pkt_params: &PacketParams,
        duty_cycle_params: Option<&DutyCycleParams>,
        rx_continuous: bool,
        rx_boosted_if_supported: bool,
        symbol_timeout: u16,
        rx_timeout_in_ms: u32,
    ) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .do_rx(
                rx_pkt_params,
                duty_cycle_params,
                rx_continuous,
                rx_boosted_if_supported,
                symbol_timeout,
                rx_timeout_in_ms,
            )
            .await
    }

    async fn get_rx_payload(
        &mut self,
        rx_pkt_params: &PacketParams,
        receiving_buffer: &mut [u8],
    ) -> Result<u8, RadioError> {
        self.0
            .deref_mut()
            .get_rx_payload(rx_pkt_params, receiving_buffer)
            .await
    }

    async fn get_rx_packet_status(&mut self) -> Result<PacketStatus, RadioError> {
        self.0.deref_mut().get_rx_packet_status().await
    }

    async fn do_cad(
        &mut self,
        mdltn_params: &ModulationParams,
        rx_boosted_if_supported: bool,
    ) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .do_cad(mdltn_params, rx_boosted_if_supported)
            .await
    }

    async fn set_irq_params(&mut self, radio_mode: Option<RadioMode>) -> Result<(), RadioError> {
        self.0.deref_mut().set_irq_params(radio_mode).await
    }

    async fn process_irq(
        &mut self,
        radio_mode: RadioMode,
        rx_continuous: bool,
        cad_activity_detected: Option<&mut bool>,
    ) -> Result<(), RadioError> {
        self.0
            .deref_mut()
            .process_irq(radio_mode, rx_continuous, cad_activity_detected)
            .await
    }
}
