// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde::Serialize;

#[derive(Serialize)]
struct OZYSDevice {
    name: String,
    id: String,
    model: String,
}

const MOCK_OZYS_ID: &str = "mock-ozys";

#[tauri::command]
async fn ozys_enumerate_devices() -> Vec<OZYSDevice> {
    let mut devices = vec![];
    devices.push(OZYSDevice {
        name: "Mock OZYS Device".to_string(),
        id: MOCK_OZYS_ID.to_string(),
        model: "OZYS V3".to_string(),
    });
    devices
}

#[tauri::command]
async fn ozys_manually_add_device(path: String) -> Result<OZYSDevice, String> {
    Err("Not implemented".to_string())
}

#[tauri::command]
async fn ozys_rename_device(device_id: String, new_name: String) -> Result<(), String> {
    Err("Not implemented".to_string())
}

#[derive(Serialize)]
struct OZYSChannel {
    name: String,
    enabled: bool,
}

#[tauri::command]
async fn ozys_get_channels(device_id: String) -> Result<[OZYSChannel; 4], String> {
    if device_id == MOCK_OZYS_ID {
        return Ok([
            OZYSChannel {
                name: "Channel 1".to_string(),
                enabled: true,
            },
            OZYSChannel {
                name: "Channel 2".to_string(),
                enabled: true,
            },
            OZYSChannel {
                name: "Channel 3".to_string(),
                enabled: true,
            },
            OZYSChannel {
                name: "Channel 4".to_string(),
                enabled: false,
            },
        ]);
    }
    Err("Not implemented".to_string())
}

#[tauri::command]
async fn ozys_rename_channel(
    device_id: String,
    channel_index: String,
    new_name: String,
) -> Result<(), String> {
    Err("Not implemented".to_string())
}

#[tauri::command]
async fn ozys_control_channel(
    device_id: String,
    channel_index: String,
    enable: bool,
) -> Result<(), String> {
    Err("Not implemented".to_string())
}

#[tauri::command]
async fn ozys_control_recording(device_id: String, record: bool) -> Result<(), String> {
    Err("Not implemented".to_string())
}

#[derive(Serialize)]
struct OZYSChannelRealtimeData {
    readings: Vec<f32>,      // len: 20
    fft_0_to_2k: Vec<f32>,   // len: 200
    fft_2k_to_20k: Vec<f32>, // len: 360
}

// UI should poll this command at 10Hz
#[tauri::command]
async fn ozys_poll_realtime_data(
    device_id: String,
) -> Result<Option<[Option<OZYSChannelRealtimeData>; 4]>, String> {
    Err("Not implemented".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            ozys_enumerate_devices,
            ozys_manually_add_device,
            ozys_rename_device,
            ozys_get_channels,
            ozys_rename_channel,
            ozys_control_channel,
            ozys_control_recording,
            ozys_poll_realtime_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
