use std::time::Instant;

use crate::PullArgs;
use anyhow::anyhow;
use anyhow::Result;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::console::rpc::OpenFileStatus, driver::serial::SplitableSerial, RpcClient,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

pub async fn pull_file(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    args: PullArgs,
) -> Result<()> {
    println!("Pulling file {}", args.file_id);
    let open_status = rpc.open_file(args.file_id).await.unwrap().status;
    if open_status != OpenFileStatus::Sucess {
        return Err(anyhow!("Failed to open file"));
    }

    let file = fs::File::create(&args.host_path).await?;
    let mut writer = BufWriter::new(file);
    let mut length = 0;
    let start_time = Instant::now();
    // let mut i = 0;
    loop {
        // println!("Reading chunk {}", i);
        let read_result = rpc.read_file().await.unwrap();
        if read_result.length == 0 {
            break;
        }
        if read_result.corrupted {
            // println!("Warning: file is corrupted {}", i);
        }
        let data = &read_result.data[..read_result.length as usize];
        writer.write_all(data).await?;
        length += data.len();
        // i += 1;
    }
    let end_time = Instant::now();
    println!(
        "Pulled {} bytes in {:?}, {}KiB/s",
        length,
        end_time - start_time,
        length as f64 / 1024.0 / (end_time - start_time).as_secs_f64()
    );

    writer.flush().await?;
    rpc.close_file().await.unwrap();

    Ok(())
}
