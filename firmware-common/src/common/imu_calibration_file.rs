use super::file_types::CALIBRATION_FILE_TYPE;
use ferraris_calibration::CalibrationInfo;
use vlfs::{AsyncReader, Crc, Flash, VLFS};

// TODO return VLFSError
pub async fn read_imu_calibration_file(fs: &VLFS<impl Flash, impl Crc>) -> Option<CalibrationInfo> {
    if let Ok(Some(file)) = fs.find_first_file(CALIBRATION_FILE_TYPE).await {
        let mut file = fs.open_file_for_read(file.id).await.unwrap();
        let mut buffer = [0u8; 156];
        let result = match file.read_slice(&mut buffer, 156).await {
            Ok((buffer, _)) => {
                let cal_info = CalibrationInfo::deserialize(buffer.try_into().unwrap());
                log_info!("{:?}", cal_info);
                Some(cal_info)
            }
            Err(_) => None,
        };
        file.close().await;
        return result;
    }

    None
}
