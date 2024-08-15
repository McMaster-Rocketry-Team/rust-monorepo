use crate::common::console::DeviceType;
use crate::common::console::OpenFileStatus;
use crate::common::console::ReadFileResult;
use crate::common::vl_device_manager::prelude::*;
use crate::create_rpc;
use crate::impl_common_rpc_trait;
use vlfs::{AsyncReader, Crc, FileID, FileReader, FileType, Flash, VLFSError, VLFSReadStatus};
use vlfs::{ConcurrentFilesIterator, VLFS};
use crate::strain_gauges::global_states::SGGlobalStates;
use embassy_sync::blocking_mutex::raw::RawMutex;
use crate::driver::sg_adc::RawSGReadingsTrait;
use crate::driver::sg_adc::SGAdcController;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;

create_rpc! {
    state<F: Flash, C: Crc, D: SysReset>(
        fs: &VLFS<F, C>,
        sys_reset: &D,
        device_serial_number: &[u8; 12],
        sg_adc_controller: &Mutex::<NoopRawMutex, impl SGAdcController>,
        states: &SGGlobalStates<impl RawMutex, impl RawSGReadingsTrait>
    ) {
        let mut realtime_sample_sub = states.realtime_sample_pubsub.subscriber().unwrap();
        let mut reader: Option<FileReader<F, C>> = None;
        let mut file_iter: Option<ConcurrentFilesIterator<F, C, Option<FileType>>> = None;
    }
    rpc 0 GetDeviceType | | -> (device_type: DeviceType) {
        GetDeviceTypeResponse {
            device_type: DeviceType::OZYS,
        }
    }
    rpc 1 WhoAmI | | -> (serial_number: [u8; 12]) {
        WhoAmIResponse {
            serial_number: device_serial_number.clone(),
        }
    }
    rpc 2 OpenFile |file_id: u64| -> (status: OpenFileStatus) {
        let status = match fs.open_file_for_read(FileID(file_id)).await {
            Ok(r) => {
                let old_reader = reader.replace(r);
                if let Some(old_reader) = old_reader {
                    old_reader.close().await;
                }
                OpenFileStatus::Sucess
            }
            Err(VLFSError::FileDoesNotExist) => OpenFileStatus::DoesNotExist,
            Err(e) => {
                log_warn!("Error opening file: {:?}", e);
                OpenFileStatus::Error
            }
        };
        OpenFileResponse { status }
    }
    rpc 3 ReadFile | | -> (result: ReadFileResult) {
        let response = if let Some(reader) = reader.as_mut() {
            let mut buffer = [0u8; 128];
            match reader.read_all(&mut buffer).await {
                Ok((read_buffer, read_status)) => ReadFileResponse {
                    result: ReadFileResult{
                        length: read_buffer.len() as u8,
                        data: buffer,
                        corrupted: matches!(read_status, VLFSReadStatus::CorruptedPage { .. }),
                    }
                },
                Err(e) => {
                    log_warn!("Error reading file: {:?}", e);
                    ReadFileResponse {
                        result: ReadFileResult{
                            length: 0,
                            data: buffer,
                            corrupted: true,
                        }
                    }
                }
            }
        } else {
            ReadFileResponse {
                result: ReadFileResult{
                    length: 0,
                    data: [0u8; 128],
                    corrupted: false,
                }
            }
        };
        response
    }
    rpc 4 CloseFile | | -> () {
        if let Some(reader) = reader.take() {
            reader.close().await;
        }
    }
    rpc 5 StartListFiles |file_type: Option<u16>| -> () {
        file_iter = Some(fs.concurrent_files_iter(file_type.map(FileType)).await);
        StartListFilesResponse {}
    }
    rpc 6 GetListedFile | | -> (file_id: Option<u64>) {
        if let Some(file_iter) = &mut file_iter {
            match file_iter.next().await {
                Ok(Some(file)) => {
                    GetListedFileResponse {
                        file_id: Some(file.id.0),
                    }
                }
                Ok(None) => {
                    GetListedFileResponse { file_id: None }
                }
                Err(_) => {
                    GetListedFileResponse { file_id: None }
                }
            }
        }else{
            GetListedFileResponse { file_id: None }
        }
    }
    rpc 7 ResetDevice | | -> () {
        sys_reset.reset();
        ResetDeviceResponse {}
    }
    rpc 8 ClearData | | -> () {
        fs.remove_files(()).await.ok();
        ClearDataResponse {}
    }
    rpc 9 SetAdcEnable |enabled: bool| -> () {
        sg_adc_controller
            .lock()
            .await
            .set_enable(enabled)
            .await;
        SetAdcEnableResponse {}
    }
    rpc 10 GetRealTime | | -> (sample: Option<[f32; 4]>) {
        GetRealTimeResponse { 
            sample: realtime_sample_sub.try_next_message_pure()
        }
    }
}

impl_common_rpc_trait!(RpcClient);