use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct OzysDeviceInfo {
    pub name: String,
    pub id: String,
    pub model: String,
}

#[derive(Serialize, Clone, Copy)]
pub enum OZYSChannelState {
    Connected,
    Disconnected,
    Disabled,
}

#[derive(Serialize, Clone)]
pub struct OzysChannelRealtimeData {
    readings: Vec<f32>,      // len: 20
    fft_0_to_2k: Vec<f32>,   // len: 200
    fft_2k_to_20k: Vec<f32>, // len: 360
}

#[async_trait]
pub trait OzysDevice {
    fn get_device_info(&self) -> OzysDeviceInfo;
    async fn rename_device(&mut self, new_name: String) -> Result<()>;

    /// this is slow, so we don't want to call it too often
    async fn get_channel_names(&mut self) -> Result<[Option<String>; 4]>;

    async fn get_channel_states(&mut self) -> Result<[OZYSChannelState; 4]>;
    async fn rename_channel(&mut self, channel_index: usize, new_name: String) -> Result<()>;
    async fn control_channel(&mut self, channel_index: usize, enabled: bool) -> Result<()>;
    async fn control_recording(&mut self, record: bool) -> Result<()>;
    async fn poll_realtime_data(&mut self) -> Result<Option<[Option<OzysChannelRealtimeData>; 4]>>;
}
