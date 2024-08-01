use crate::PullArgs;
use anyhow::anyhow;
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{common::console::rpc::OpenFileStatus, driver::serial::SplitableSerial, RpcClient};
use std::fs;
use std::io::Write;

pub async fn pull_file(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    args: PullArgs,
) -> Result<()> {
    let open_status = rpc.open_file(args.file_id).await.unwrap().status;
    if open_status != OpenFileStatus::Sucess {
        return Err(anyhow!("Failed to open file"));
    }

    let mut file = fs::File::create(&args.host_path)?;
    loop {
        let read_result = rpc.read_file().await.unwrap();
        if read_result.length == 0 {
            break;
        }
        if read_result.corrupted {
            println!("Warning: file is corrupted");
        }
        let data = &read_result.data[..read_result.length as usize];
        file.write_all(data)?;
    }

    rpc.close_file().await.unwrap();
    file.flush()?;

    Ok(())
}
