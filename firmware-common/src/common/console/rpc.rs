use core::cell::RefCell;

use vlfs::{AsyncReader, Crc, FileID, FileReader, FileType, Flash, VLFSError, VLFSReadStatus};

use crate::common::device_manager::prelude::*;
use crate::create_rpc;

create_rpc! {
    enums {
        enum DeviceModel {
            VLF1,
            VLF2,
            VLF3,
            VLF4,
        }
        enum OpenFileStatus {
            Sucess,
            DoesNotExist,
            Error,
        }
    }
    state<F: Flash, C: Crc>(services: &SystemServices<'_,'_,'_, impl DelayNs + Copy, impl Clock, F, C>) {
        let fs = &services.fs;
        let mut reader: Option<FileReader<F, C>> = None;

        let selected_file_type: RefCell<Option<FileType>> = RefCell::new(None);
        let mut file_iter = fs.concurrent_files_iter_filter(|file| {
            let borrowed = selected_file_type.borrow();
            if let Some(file_type) = borrowed.as_ref() {
                file.typ == *file_type
            } else {
                true
            }
        })
        .await;
    }
    rpc 0 WhoAmI | | -> (name: [u8; 64], model: DeviceModel, serial_number: [u8; 12]) {
        // TODO
        let mut name = [0u8; 64];
        name[..5].copy_from_slice(b"VLF4\0");
        WhoAmIResponse {
            name: name,
            model: DeviceModel::VLF4,
            serial_number: [69u8; 12],
        }
    }
    rpc 1 OpenFile |file_id: u64| -> (status: OpenFileStatus) {
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
    rpc 2 ReadFile | | -> (data: [u8; 128], length: u8, corrupted: bool) {
        let response = if let Some(reader) = reader.as_mut() {
            let mut buffer = [0u8; 128];
            match reader.read_all(&mut buffer).await {
                Ok((read_buffer, read_status)) => ReadFileResponse {
                    length: read_buffer.len() as u8,
                    data: buffer,
                    corrupted: matches!(read_status, VLFSReadStatus::CorruptedPage { .. }),
                },
                Err(e) => {
                    log_warn!("Error reading file: {:?}", e);
                    ReadFileResponse {
                        length: 0,
                        data: buffer,
                        corrupted: true,
                    }
                }
            }
        } else {
            ReadFileResponse {
                length: 0,
                data: [0u8; 128],
                corrupted: false,
            }
        };
        response
    }
    rpc 3 CloseFile | | -> () {
        if let Some(reader) = reader.take() {
            reader.close().await;
        }
    }
    rpc 4 StartListFiles |file_type: Option<u16>| -> () {
        selected_file_type.replace(file_type.map(FileType));
        file_iter.reset();
        StartListFilesResponse {}
    }
    rpc 5 GetListedFile | | -> (file_id: Option<u64>) {
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
    }
}
