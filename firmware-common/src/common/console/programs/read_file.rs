use core::cell::RefCell;

use defmt::warn;
use vlfs::{AsyncReader, Crc, FileID, FileReader, Flash, VLFSError, VLFSReadStatus, VLFS};

use crate::{common::console::console_program::ConsoleProgram, create_rpc, driver::serial::Serial};
use crate::device_manager_type;
use crate::common::device_manager::prelude::*;

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
}

pub struct ReadFile<'a, F: Flash, C: Crc> {
    vlfs: &'a VLFS<F, C>,
}

impl<'a, F: Flash, C: Crc> ReadFile<'a, F, C>  {
    pub fn new(vlfs: &'a VLFS<F, C>) -> Self {
        Self {vlfs}
    }
}

impl<'a, F: Flash, C: Crc> ConsoleProgram for ReadFile<'a, F,C>{
    fn id(&self) -> u64 {
        0x4
    }

    async fn run(&mut self, serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        let reader: RefCell<Option<FileReader<F, C>>> = RefCell::new(None);
        let result = run_rpc_server(
            serial,
            async |file_id| {
                let status = match self.vlfs.open_file_for_read(FileID(file_id)).await {
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
                response
            },
            async || {
                let mut borrowed = reader.borrow_mut();

                if let Some(reader) = borrowed.take() {
                    reader.close().await;
                }
            },
        )
        .await;
        if let Err(e) = result {
            warn!("rpc ended due to {:?}", e);
        }
    }
}