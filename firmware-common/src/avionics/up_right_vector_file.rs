use crate::common::files::AVIONICS_UP_RIGHT_FILE_TYPE;
use nalgebra::Vector3;
use vlfs::io_traits::AsyncWriter;
use vlfs::{io_traits::AsyncReader, Crc, Flash, LsFileEntry, VLFSError, VLFS};

pub async fn read_up_right_vector(fs: &VLFS<impl Flash, impl Crc>) -> Option<Vector3<f32>> {
    let file = fs.find_file_by_type(AVIONICS_UP_RIGHT_FILE_TYPE).await;
    if let Some(LsFileEntry { file_id, .. }) = file {
        if let Ok(mut reader) = fs.open_file_for_read(file_id).await {
            let mut buffer = [0u8; 12];
            let result = match reader.read_slice(&mut buffer, 12).await {
                Ok((buffer, _)) if buffer.len() == 12 => {
                    log_info!("Read up right vector successfully");
                    Some(Vector3::new(
                        f32::from_be_bytes((&buffer[0..4]).try_into().unwrap()),
                        f32::from_be_bytes((&buffer[4..8]).try_into().unwrap()),
                        f32::from_be_bytes((&buffer[8..12]).try_into().unwrap()),
                    ))
                }
                _ => {
                    log_info!("Failed to read up right vector");
                    None
                }
            };
            reader.close().await;
            return result;
        }
    }
    None
}

pub async fn write_up_right_vector<F: Flash>(
    fs: &VLFS<F, impl Crc>,
    vector: Vector3<f32>,
) -> Result<(), VLFSError<F::Error>> {
    fs.remove_files(|file_entry| file_entry.file_type == AVIONICS_UP_RIGHT_FILE_TYPE)
        .await?;

    let mut buffer = [0u8; 12];
    buffer[0..4].copy_from_slice(&vector.x.to_be_bytes());
    buffer[4..8].copy_from_slice(&vector.y.to_be_bytes());
    buffer[8..12].copy_from_slice(&vector.z.to_be_bytes());

    let file_id = fs.create_file(AVIONICS_UP_RIGHT_FILE_TYPE).await?;
    let mut writer = fs.open_file_for_write(file_id).await?;
    writer.extend_from_slice(&buffer).await?;
    writer.close().await?;
    log_info!("up right vector saved");
    Ok(())
}
