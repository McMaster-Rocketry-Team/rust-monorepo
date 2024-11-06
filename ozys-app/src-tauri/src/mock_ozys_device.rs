use crate::ozys_device::{OZYSChannelState, OzysChannelRealtimeData, OzysDevice, OzysDeviceInfo};
use anyhow::Result;
use async_trait::async_trait;

pub struct MockOzysDevice {
    device_info: OzysDeviceInfo,
}

impl MockOzysDevice {
    pub fn new(name: Option<String>) -> Self {
        let name = name.unwrap_or("Mock OZYS Device".to_string());
        Self {
            device_info: OzysDeviceInfo {
                name: name.clone(),
                id: name.to_lowercase().replace(" ", "-"),
                model: "OZYS V3".to_string(),
                channels: vec![
                    OZYSChannelState::Connected {
                        enabled: true,
                        name: "Channel 1".to_string(),
                        id: "channel-1".into(),
                    },
                    OZYSChannelState::Connected {
                        enabled: true,
                        name: "Channel 2".to_string(),
                        id: "channel-2".into(),
                    },
                    OZYSChannelState::Connected {
                        enabled: false,
                        name: "Channel 3".to_string(),
                        id: "channel-3".into(),
                    },
                    OZYSChannelState::Disconnected,
                ],
            },
        }
    }
}

#[async_trait]
impl OzysDevice for MockOzysDevice {
    fn get_device_info(&self) -> OzysDeviceInfo {
        self.device_info.clone()
    }

    async fn rename_device(&mut self, new_name: String) -> Result<()> {
        self.device_info.name = new_name;
        Ok(())
    }

    async fn rename_channel(&mut self, channel_index: usize, new_name: String) -> Result<()> {
        if let OZYSChannelState::Connected { name, .. } =
            &mut self.device_info.channels[channel_index]
        {
            *name = new_name;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Channel {} is not connected",
                channel_index
            ))
        }
    }

    async fn control_channel(&mut self, channel_index: usize, new_enabled: bool) -> Result<()> {
        if let OZYSChannelState::Connected { enabled, .. } =
            &mut self.device_info.channels[channel_index]
        {
            *enabled = new_enabled;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Channel {} is not connected",
                channel_index
            ))
        }
    }

    async fn control_recording(&mut self, _record: bool) -> Result<()> {
        Ok(())
    }

    async fn poll_realtime_data(&mut self) -> Result<Option<Vec<Option<OzysChannelRealtimeData>>>> {
        Ok(Some(
            self.device_info
                .channels
                .iter()
                .map(|state| {
                    if matches!(state, OZYSChannelState::Connected { enabled: true, .. }) {
                        Some(OzysChannelRealtimeData {
                            readings: vec![0.0; 10],
                            reading_noises: vec![0.0; 10],
                            fft_0_to_2k: vec![0.0; 200],
                            fft_2k_to_20k: vec![0.0; 360],
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        ))
    }
}
