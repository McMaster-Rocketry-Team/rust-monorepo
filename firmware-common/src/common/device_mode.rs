use vlfs::{VLFS, Flash, Crc, VLFSError};
use vlfs::io_traits::{AsyncReader, AsyncWriter};


#[derive(Clone, Copy, defmt::Format)]
#[repr(u8)]
pub enum DeviceMode {
    Avionics = 1,
    GCM = 2,
    BeaconSender = 3,
    BeaconReceiver = 4,
}

impl TryFrom<u8> for DeviceMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DeviceMode::Avionics),
            2 => Ok(DeviceMode::GCM),
            3 => Ok(DeviceMode::BeaconSender),
            4 => Ok(DeviceMode::BeaconReceiver),
            _ => Err(()),
        }
    }
}

static DEVICE_MODE_FILE_ID: u64 = 0;
static DEVICE_MODE_FILE_TYPE: u16 = 0;

pub async fn read_device_mode(fs: &VLFS<impl Flash, impl Crc>) -> Option<DeviceMode> {
    if let Ok(mut reader) = fs.open_file_for_read(DEVICE_MODE_FILE_ID).await {
        let mut file_content = [0u8; 1];
        let read_result = reader.read_all(&mut file_content).await;
        reader.close().await;
        if let Ok(file_content) = read_result {
            if file_content.len() != 1 {
                return None;
            }
            return file_content[0].try_into().ok();
        }
    }
    None
}

pub async fn write_device_mode<F: Flash>(fs: &VLFS<F, impl Crc>, mode: DeviceMode) -> Result<(), VLFSError<F>> {
    if fs.exists(DEVICE_MODE_FILE_ID).await {
        fs.remove_file(DEVICE_MODE_FILE_ID).await?;
    }

    fs.create_file(DEVICE_MODE_FILE_ID, DEVICE_MODE_FILE_TYPE).await?;
    let mut writer = fs.open_file_for_write(DEVICE_MODE_FILE_ID).await?;
    writer.extend_from_slice(&[mode as u8]).await?;
    writer.close().await?;
    Ok(())
}
