use vlfs::{FileID, FileType};

use crate::driver::serial::SplitableSerial;

use super::{create_rpc::RpcClientError, DeviceType, OpenFileStatus, ReadFileResult};

pub trait CommonRPCTrait<S: SplitableSerial> {
    async fn get_device_type(&mut self) -> Result<DeviceType, RpcClientError<S>>;
    async fn open_file(&mut self, file_id: FileID) -> Result<OpenFileStatus, RpcClientError<S>>;
    async fn read_file(&mut self) -> Result<ReadFileResult, RpcClientError<S>>;
    async fn close_file(&mut self) -> Result<(), RpcClientError<S>>;

    async fn start_list_files(
        &mut self,
        file_type: Option<FileType>,
    ) -> Result<(), RpcClientError<S>>;
    async fn get_listed_file(&mut self) -> Result<Option<FileID>, RpcClientError<S>>;
}

#[macro_export]
macro_rules! impl_common_rpc_trait {
    ($rpc_client: ident) => {
        impl<'a, S: SplitableSerial, D: embedded_hal_async::delay::DelayNs> crate::common::console::common_rpc_trait::CommonRPCTrait<S> for $rpc_client<'a, S, D> {
            async fn get_device_type(&mut self) -> Result<DeviceType, crate::common::console::create_rpc::RpcClientError<S>> {
                self.get_device_type()
                    .await
                    .map(|response| response.device_type)
            }
        
            async fn open_file(&mut self, file_id: FileID) -> Result<OpenFileStatus, crate::common::console::create_rpc::RpcClientError<S>> {
                self.open_file(file_id.0)
                    .await
                    .map(|response| response.status)
            }
        
            async fn read_file(&mut self) -> Result<ReadFileResult, crate::common::console::create_rpc::RpcClientError<S>> {
                self.read_file().await.map(|response| response.result)
            }
        
            async fn close_file(&mut self) -> Result<(), crate::common::console::create_rpc::RpcClientError<S>> {
                self.close_file().await.map(|_| ())
            }
        
            async fn start_list_files(
                &mut self,
                file_type: Option<FileType>,
            ) -> Result<(), crate::common::console::create_rpc::RpcClientError<S>> {
                self.start_list_files(file_type.map(|t| t.0))
                    .await
                    .map(|_| ())
            }
        
            async fn get_listed_file(&mut self) -> Result<Option<FileID>, crate::common::console::create_rpc::RpcClientError<S>> {
                self.get_listed_file()
                    .await
                    .map(|response| response.file_id.map(FileID))
            }
        }
        
    }
}