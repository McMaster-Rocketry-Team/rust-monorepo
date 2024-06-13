use core::cell::RefCell;

use vlfs::{
    AsyncReader, ConcurrentFilesIterator, Crc, FileEntry, FileID, FileReader, FileType, Flash,
    VLFSError, VLFSReadStatus, VLFS,
};

use crate::{create_rpc, SplitableSerial};

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
    rpc 0 WhoAmI {
        request()
        response(name: [u8; 64], model: DeviceModel, serial_number: [u8; 12])
    }
    rpc 1 OpenFile {
        request(file_id: u64)
        response(status: OpenFileStatus)
    }
    rpc 2 ReadFile {
        request()
        response(data: [u8; 128], length: u8, corrupted: bool)
    }
    rpc 3 CloseFile {
        request()
        response()
    }
    rpc 4 StartListFiles {
        request(file_type: u16)
        response()
    }
    rpc 5 GetListedFile {
        request()
        response(file_id: Option<u64>)
    }
}

pub async fn run_console<F: Flash, C: Crc>(serial: &mut impl SplitableSerial, fs: &VLFS<F, C>) {
    let reader: RefCell<Option<FileReader<F, C>>> = RefCell::new(None);

    let selected_file_type: RefCell<Option<FileType>> = RefCell::new(None);
    let file_iter: RefCell<Option<ConcurrentFilesIterator<F, C, fn(&FileEntry) -> bool>>> =
        RefCell::new(None);

    let result = run_rpc_server(
        serial,
        async || {
            // TODO
            let mut name = [0u8; 64];
            name[..5].copy_from_slice(b"VLF4\0");
            return WhoAmIResponse {
                name: name,
                model: DeviceModel::VLF4,
                serial_number: [0u8; 12],
            };
        },
        async |file_id| {
            let status = match fs.open_file_for_read(FileID(file_id)).await {
                Ok(r) => {
                    let old_reader = reader.replace(Some(r));
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
        },
        async || {
            let mut borrowed = reader.borrow_mut();
            let response = if let Some(reader) = borrowed.as_mut() {
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
        },
        async || {
            let mut borrowed = reader.borrow_mut();

            if let Some(reader) = borrowed.take() {
                reader.close().await;
            }

            CloseFileResponse {}
        },
        async |file_type| {
            selected_file_type.replace(Some(FileType(file_type)));
            file_iter.replace(Some(fs.concurrent_files_iter().await));
            StartListFilesResponse {}
        },
        async || {
            if selected_file_type.borrow().is_none() {
                return GetListedFileResponse { file_id: None };
            }
            let file_type = selected_file_type.borrow().unwrap();

            let mut file_iter = file_iter.borrow_mut();
            let file_iter = file_iter.as_mut().unwrap();
            loop {
                match file_iter.next().await {
                    Ok(Some(file)) => {
                        if file.typ == file_type {
                            return GetListedFileResponse {
                                file_id: Some(file.id.0),
                            };
                        } else {
                            continue;
                        }
                    }
                    Ok(None) => {
                        return GetListedFileResponse { file_id: None };
                    }
                    Err(_) => {
                        return GetListedFileResponse { file_id: None };
                    }
                }
            }
        },
    )
    .await;

    if let Err(e) = result {
        log_error!("Error running console: {:?}", e);
    }
}
