use crate::LSArgs;
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{driver::serial::SplitableSerial, RpcClient};

pub async fn list_files(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    args: LSArgs,
) -> Result<Vec<u64>> {
    let mut result = Vec::new();

    rpc.start_list_files(args.file_type).await.unwrap();
    loop {
        let response = rpc.get_listed_file().await.unwrap();
        if let Some(file_id) = response.file_id {
            result.push(file_id);
        } else {
            break;
        }
    }

    Ok(result)
}
