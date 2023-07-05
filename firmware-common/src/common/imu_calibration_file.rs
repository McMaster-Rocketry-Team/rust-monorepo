use super::files::CALIBRATION_FILE_TYPE;
use defmt::unwrap;
use ferraris_calibration::CalibrationInfo;
use vlfs::io_traits::AsyncReader;
use vlfs::{Crc, Flash, VLFS};

pub async fn read_imu_calibration_file(fs: &VLFS<impl Flash, impl Crc>) -> Option<CalibrationInfo> {
    if let Some(file_entry) = fs.find_file_by_type(CALIBRATION_FILE_TYPE).await {
        let mut file = unwrap!(fs.open_file_for_read(file_entry.file_id).await);
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
