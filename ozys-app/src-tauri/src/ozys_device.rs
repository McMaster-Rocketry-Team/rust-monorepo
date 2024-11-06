use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct OzysDeviceInfo {
    pub name: String,
    pub id: String,
    pub model: String,
    pub channels: Vec<OZYSChannelState>,
}

#[derive(Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "state")]
pub enum OZYSChannelState {
    Disconnected,
    Connected {
        enabled: bool,
        name: String,
        id: String,
    },
}

/// The frontend polls this at 10Hz, contains the readings and
/// FFT of the measurements from the last 100ms
#[derive(Serialize, Clone)]
pub struct OzysChannelRealtimeData {
    /// Absolute reading values
    /// Unit is 1, e.g. 0.01 = 1% of length change
    /// 10 readings long, interval between readings is 10ms
    pub readings: Vec<f32>,

    /// Standard deviation of all the samples inside the 10ms interval
    /// 10 values long, one value for each reading
    pub reading_noises: Vec<f32>,

    /// FFT for frequencies below 2kHz, with 10Hz resolution
    /// 200 values long
    /// e.g. fft_0_to_2k[0] is the power of the 0-10Hz band in the last 100ms
    ///      fft_0_to_2k[1] is the power of the 10-20Hz band in the last 100ms
    pub fft_0_to_2k: Vec<f32>,

    /// FFT for frequencies between 2kHz and 20kHz, with 50Hz resolution
    /// 360 values long
    /// e.g. fft_2k_to_20k[0] is the power of the 2k-2.05k band in the last 100ms
    pub fft_2k_to_20k: Vec<f32>,
}

#[async_trait]
pub trait OzysDevice {
    fn get_device_info(&self) -> OzysDeviceInfo;
    async fn rename_device(&mut self, new_name: String) -> Result<()>;
    async fn rename_channel(&mut self, channel_index: usize, new_name: String) -> Result<()>;
    async fn control_channel(&mut self, channel_index: usize, enabled: bool) -> Result<()>;
    async fn control_recording(&mut self, record: bool) -> Result<()>;

    /// The frontend should poll this at 10Hz
    /// Returns Ok(None) if polling is too frequent
    async fn poll_realtime_data(&mut self) -> Result<Option<Vec<Option<OzysChannelRealtimeData>>>>;
}
