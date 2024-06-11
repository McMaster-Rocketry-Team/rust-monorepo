use core::cell::RefCell;

use defmt::warn;
use vlfs::{AsyncReader, Crc, FileID, FileReader, Flash, VLFSError, VLFSReadStatus, VLFS};

use crate::{create_rpc, driver::serial::Serial};

create_rpc! {
    enums {
        enum OpenFileStatus {
            Sucess,
            DoesNotExist,
            Error,
        }
    }
    rpc 0 OpenFile {
        request(file_id: u64)
        response(status: OpenFileStatus)
    }
    rpc 1 ReadFile {
        request()
        response(data: [u8; 128], length: u8, corrupted: bool)
    }
    rpc 2 CloseFile {
        request()
        response()
    }
}

// TODO implement `ConsoleProgram` and add to `start_common_programs`
pub struct ReadFile {}

impl ReadFile {
    pub fn new() -> Self {
        Self {}
    }

    pub fn id(&self) -> u64 {
        0x4
    }

    pub async fn start<T: Serial, F: Flash, C: Crc>(
        &self,
        serial: &mut T,
        vlfs: &VLFS<F, C>,
    ) -> Result<(), ()> {
        let reader: RefCell<Option<FileReader<F, C>>> = RefCell::new(None);
        let result = run_rpc_server(
            serial,
            async |file_id| {
                let status = match vlfs.open_file_for_read(FileID(file_id)).await {
                    Ok(r) => {
                        reader.replace(Some(r));
                        OpenFileStatus::Sucess
                    }
                    Err(VLFSError::FileDoesNotExist) => OpenFileStatus::DoesNotExist,
                    Err(e) => {
                        warn!("Error opening file: {:?}", e);
                        OpenFileStatus::Error
                    }
                };
                (OpenFileResponse { status }, false)
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
                            warn!("Error reading file: {:?}", e);
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
                (response, false)
            },
            async || {
                let mut borrowed = reader.borrow_mut();

                if let Some(reader) = borrowed.take() {
                    reader.close().await;
                }

                (CloseFileResponse {}, true)
            },
        )
        .await;
        if let Err(e) = result {
            warn!("rpc ended due to {:?}", e);
        }
        Ok(())
    }
}
