// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use mock_ozys_device::MockOzysDevice;
use ozys_device::{OzysChannelRealtimeData, OzysDevice, OzysDeviceInfo};
use tauri::Manager as _;
use tauri::State;
use tokio::sync::Mutex;

mod mock_ozys_device;
mod ozys_device;

#[derive(Default)]
struct AppState {
    connected_ozys_devices: Vec<Box<dyn OzysDevice + Sync + Send>>,
}

impl AppState {
    fn get_device_by_id(
        &mut self,
        device_id: &str,
    ) -> Result<&mut Box<dyn OzysDevice + Sync + Send>, String> {
        self.connected_ozys_devices
            .iter_mut()
            .find(|d| d.get_device_info().id == device_id)
            .ok_or("Device not found".to_string())
    }
}

#[tauri::command]
async fn ozys_enumerate_devices(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<OzysDeviceInfo>, String> {
    let mut state = state.lock().await;
    if state.connected_ozys_devices.is_empty() {
        state
            .connected_ozys_devices
            .push(Box::new(MockOzysDevice::new()));
    }
    Ok(state
        .connected_ozys_devices
        .iter_mut()
        .map(|d| d.get_device_info())
        .collect())
}

#[tauri::command]
async fn ozys_manually_add_device(path: String) -> Result<OzysDeviceInfo, String> {
    Err("Not implemented".to_string())
}

#[tauri::command]
async fn ozys_rename_device(
    state: State<'_, Mutex<AppState>>,
    device_id: String,
    new_name: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    let device = state.get_device_by_id(&device_id)?;
    device
        .rename_device(new_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn ozys_rename_channel(
    state: State<'_, Mutex<AppState>>,
    device_id: String,
    channel_index: usize,
    new_name: String,
) -> Result<(), String> {
    let mut state = state.lock().await;
    let device = state.get_device_by_id(&device_id)?;
    device
        .rename_channel(channel_index, new_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn ozys_control_channel(
    state: State<'_, Mutex<AppState>>,
    device_id: String,
    channel_index: usize,
    enable: bool,
) -> Result<(), String> {
    let mut state = state.lock().await;
    let device = state.get_device_by_id(&device_id)?;
    device
        .control_channel(channel_index, enable)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn ozys_control_recording(
    state: State<'_, Mutex<AppState>>,
    device_id: String,
    record: bool,
) -> Result<(), String> {
    let mut state = state.lock().await;
    let device = state.get_device_by_id(&device_id)?;
    device
        .control_recording(record)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// UI should poll this command at 10Hz
#[tauri::command]
async fn ozys_poll_realtime_data(
    state: State<'_, Mutex<AppState>>,
    device_id: String,
) -> Result<Option<Vec<Option<OzysChannelRealtimeData>>>, String> {
    let mut state = state.lock().await;
    let device = state.get_device_by_id(&device_id)?;
    device.poll_realtime_data().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState::default()));
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            ozys_enumerate_devices,
            ozys_manually_add_device,
            ozys_rename_device,
            ozys_rename_channel,
            ozys_control_channel,
            ozys_control_recording,
            ozys_poll_realtime_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
