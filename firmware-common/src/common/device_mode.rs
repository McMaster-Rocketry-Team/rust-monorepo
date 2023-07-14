use vlfs::io_traits::{AsyncReader, AsyncWriter};
use vlfs::{Crc, Flash, LsFileEntry, VLFSError, VLFS};
#[cfg(feature = "clap")] 
use clap::ValueEnum;
use super::files::DEVICE_MODE_FILE_TYPE;

#[cfg_attr(feature = "clap", derive(ValueEnum))]
#[derive(Clone, Copy, Debug, defmt::Format)]
#[repr(u8)]
pub enum DeviceMode {
    Avionics = 1,
    GCM = 2,
    BeaconSender = 3,
    BeaconReceiver = 4,
    GroundTestAvionics = 5,
    GroundTestGCM = 6,
}

impl TryFrom<u8> for DeviceMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DeviceMode::Avionics),
            2 => Ok(DeviceMode::GCM),
            3 => Ok(DeviceMode::BeaconSender),
            4 => Ok(DeviceMode::BeaconReceiver),
            5 => Ok(DeviceMode::GroundTestAvionics),
            6 => Ok(DeviceMode::GroundTestGCM),
            _ => Err(()),
        }
    }
}

pub async fn read_device_mode(fs: &VLFS<impl Flash, impl Crc>) -> Option<DeviceMode> {
    let mut files_iter = fs.files_iter(Some(DEVICE_MODE_FILE_TYPE)).await;
    let file = files_iter.next();
    drop(files_iter);
    if let Some(LsFileEntry {
        file_id,
        ..
    }) = file
    {
        if let Ok(mut reader) = fs.open_file_for_read(file_id).await {
            let mut buffer = [0u8; 1];
            let read_result = reader.read_u8(&mut buffer).await;
            reader.close().await;
            if let Ok((Some(value), _)) = read_result {
                return value.try_into().ok();
            }
        }
    }
    None
}

pub async fn write_device_mode<F: Flash>(
    fs: &VLFS<F, impl Crc>,
    mode: DeviceMode,
) -> Result<(), VLFSError<F::Error>> {
    fs.remove_files(|file_entry| file_entry.file_type == DEVICE_MODE_FILE_TYPE)
        .await?;

    let file_id = fs.create_file(DEVICE_MODE_FILE_TYPE).await?;
    let mut writer = fs.open_file_for_write(file_id).await?;
    writer.extend_from_slice(&[mode as u8]).await?;
    writer.close().await?;
    Ok(())
}
