use std::path::PathBuf;
use std::time::Instant;

use anyhow::anyhow;
use anyhow::Result;
use firmware_common::{
    common::console::OpenFileStatus, driver::serial::SplitableSerial, CommonRPCTrait,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use vlfs::FileID;
use vlfs::FileType;

use super::list_files;

pub async fn pull_file<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    file_id: FileID,
    host_path: PathBuf,
) -> Result<()> {
    println!("Pulling file {}", file_id.0);
    let open_status = rpc.open_file(file_id).await.unwrap();
    if open_status != OpenFileStatus::Sucess {
        return Err(anyhow!("Failed to open file"));
    }

    let file = fs::File::create(&host_path).await?;
    let mut writer = BufWriter::new(file);
    let mut length = 0;
    let start_time = Instant::now();
    // TODO ignore corrupted chunk if its the last chunk
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

pub async fn pull_files<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    file_type: FileType,
    file_type_name: &str,
    file_type_extension: &str,
    save_folder: &PathBuf,
) -> Result<Vec<PathBuf>> {
    let file_ids = list_files(rpc, Some(file_type)).await?;
    let mut pulled_file_paths = vec![];

    for file_id in file_ids {
        let mut file_path = save_folder.clone();
        file_path.push(format!(
            "{}.{}.{}",
            file_id.0, file_type_name, file_type_extension
        ));
        pulled_file_paths.push(file_path.clone());
        pull_file(rpc, file_id, file_path).await?;
    }

    Ok(pulled_file_paths)
}
