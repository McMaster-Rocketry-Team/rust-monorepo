use crate::ozys_device::{OZYSChannelState, OzysChannelRealtimeData, OzysDevice, OzysDeviceInfo};
use anyhow::Result;
use async_trait::async_trait;

pub struct MockOzysDevice {
    device_info: OzysDeviceInfo,
    channel_names: [Option<String>; 4],
    channel_states: [OZYSChannelState; 4],
}

impl MockOzysDevice {
    pub fn new() -> Self {
        Self {
            device_info: OzysDeviceInfo {
                name: "Mock OZYS Device".to_string(),
                id: "mock-ozys-id".to_string(),
                model: "OZYS V3".to_string(),
            },
            channel_names: [
                Some("Channel 1".to_string()),
                Some("Channel 2".to_string()),
                Some("Channel 3".to_string()),
                None,
            ],
            channel_states: [
                OZYSChannelState::Connected,
                OZYSChannelState::Connected,
                OZYSChannelState::Disabled,
                OZYSChannelState::Disconnected,
            ],
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

    async fn get_channel_names(&mut self) -> Result<[Option<String>; 4]> {
        Ok(self.channel_names.clone())
    }

    async fn get_channel_states(&mut self) -> Result<[OZYSChannelState; 4]> {
        Ok(self.channel_states.clone())
    }

    async fn rename_channel(&mut self, channel_index: usize, new_name: String) -> Result<()> {
        if let Some(channel_name) = &mut self.channel_names[channel_index] {
            *channel_name = new_name;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Channel {} is not connected",
                channel_index
            ))
        }
    }

    async fn control_channel(&mut self, channel_index: usize, enabled: bool) -> Result<()> {
        let channel_state = &mut self.channel_states[channel_index];

        match (&channel_state, enabled) {
            (OZYSChannelState::Connected, false) => {
                *channel_state = OZYSChannelState::Disabled;
                Ok(())
            }
            (OZYSChannelState::Disabled, true) => {
                *channel_state = OZYSChannelState::Connected;
                Ok(())
            }
            (OZYSChannelState::Disconnected, _) => Err(anyhow::anyhow!(
                "Channel {} is not connected",
                channel_index
            )),
            _ => Ok(()),
        }
    }

    async fn control_recording(&mut self, _record: bool) -> Result<()> {
        Ok(())
    }

    async fn poll_realtime_data(&mut self) -> Result<Option<[Option<OzysChannelRealtimeData>; 4]>> {
        Ok(Some(self.channel_states.map(|state| {
            if state == OZYSChannelState::Connected {
                Some(OzysChannelRealtimeData {
                    readings: vec![0.0; 10],
                    reading_noises: vec![0.0; 10],
                    fft_0_to_2k: vec![0.0; 200],
                    fft_2k_to_20k: vec![0.0; 360],
                })
            } else {
                None
            }
        })))
    }
}
