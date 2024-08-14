use std::time::Instant;

use crate::PullArgs;
use anyhow::anyhow;
use anyhow::Result;
use firmware_common::{
    common::console::OpenFileStatus, driver::serial::SplitableSerial, CommonRPCTrait,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

pub async fn pull_file<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    args: PullArgs,
) -> Result<()> {
    println!("Pulling file {}", args.file_id.0);
    let open_status = rpc.open_file(args.file_id).await.unwrap();
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
