use embedded_hal_async::delay::DelayUs;
use lora_phy::{
    mod_params::{BoardType, ModulationParams, PacketParams, PacketStatus, RadioError, RadioMode},
    mod_traits::RadioKind,
};

// TODO upstream this struct
pub struct DummyRadioKind {}

impl RadioKind for DummyRadioKind {
    fn get_board_type(&self) -> BoardType {
        BoardType::GenericSx1261
    }

    async fn reset(&mut self, _delay: &mut impl DelayUs) -> Result<(), RadioError> {
        Ok(())
    }

    async fn ensure_ready(&mut self, _mode: RadioMode) -> Result<(), RadioError> {
        Ok(())
    }

    async fn init_rf_switch(&mut self) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_standby(&mut self) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_sleep(
        &mut self,
        _delay: &mut impl embedded_hal_async::delay::DelayUs,
    ) -> Result<bool, RadioError> {
        Ok(true)
    }

    async fn set_lora_modem(&mut self, _enable_public_network: bool) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_oscillator(&mut self) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_regulator_mode(&mut self) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_tx_rx_buffer_base_address(
        &mut self,
        _tx_base_addr: usize,
        _rx_base_addr: usize,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_tx_power_and_ramp_time(
        &mut self,
        _output_power: i32,
        _mdltn_params: Option<&ModulationParams>,
        _tx_boosted_if_possible: bool,
        _is_tx_prep: bool,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn update_retention_list(&mut self) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_modulation_params(
        &mut self,
        _mdltn_params: &ModulationParams,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_packet_params(&mut self, _pkt_params: &PacketParams) -> Result<(), RadioError> {
        Ok(())
    }

    async fn calibrate_image(&mut self, _frequency_in_hz: u32) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_channel(&mut self, _frequency_in_hz: u32) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_payload(&mut self, _payload: &[u8]) -> Result<(), RadioError> {
        Ok(())
    }

    async fn do_tx(&mut self, _timeout_in_ms: u32) -> Result<(), RadioError> {
        Ok(())
    }

    async fn do_rx(
        &mut self,
        _rx_pkt_params: &lora_phy::mod_params::PacketParams,
        _duty_cycle_params: Option<&lora_phy::mod_params::DutyCycleParams>,
        _rx_continuous: bool,
        _rx_boosted_if_supported: bool,
        _symbol_timeout: u16,
        _rx_timeout_in_ms: u32,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn get_rx_payload(
        &mut self,
        _rx_pkt_params: &lora_phy::mod_params::PacketParams,
        _receiving_buffer: &mut [u8],
    ) -> Result<u8, RadioError> {
        Ok(0)
    }

    async fn get_rx_packet_status(&mut self) -> Result<PacketStatus, RadioError> {
        Ok(PacketStatus { snr: 0, rssi: 0 })
    }

    async fn do_cad(
        &mut self,
        _mdltn_params: &lora_phy::mod_params::ModulationParams,
        _rx_boosted_if_supported: bool,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn set_irq_params(
        &mut self,
        _radio_mode: Option<lora_phy::mod_params::RadioMode>,
    ) -> Result<(), RadioError> {
        Ok(())
    }

    async fn process_irq(
        &mut self,
        _radio_mode: lora_phy::mod_params::RadioMode,
        _rx_continuous: bool,
        _cad_activity_detected: Option<&mut bool>,
    ) -> Result<(), RadioError> {
        Ok(())
    }
}
