use embedded_hal_async::delay::DelayNs;
use firmware_common::{
    common::serialized_enum::SerializedEnumReader, driver::serial::SplitableSerial, RpcClient,
};
use std::path::PathBuf;
use tokio::{fs::File, io::{AsyncWriteExt, BufReader, BufWriter}};

use anyhow::Result;

use crate::{pull_file::pull_file, reader::BufReaderWrapper, PullArgs};

pub async fn pull_logs<SR: SerializedEnumReader<BufReaderWrapper<File>>>(
    rpc: &mut RpcClient<'_, impl SplitableSerial, impl DelayNs>,
    save_folder: PathBuf,
    file_id: u64,
    file_type_name: &str,
    combined_log_writer: &mut BufWriter<File>
) -> Result<()> {
    // VLL: void lake log
    let mut vll_path = save_folder.clone();
    vll_path.push(format!("{}.{}.vll", file_id, file_type_name));
    pull_file(
        rpc,
        PullArgs {
            file_id,
            host_path: vll_path.clone(),
        },
    )
    .await?;

    let mut log_path = save_folder.clone();
    log_path.push(format!("{}.{}.log", file_id, file_type_name));
    let reader = BufReader::new(File::open(vll_path).await?);
    let reader = BufReaderWrapper(reader);
    let mut reader = SR::new(reader);
    let mut writer = BufWriter::new(File::create(log_path).await?);
    while let Some(log) = reader.read_next().await.unwrap() {
        writer.write_all(format!("{:?}\n", log).as_bytes()).await?;
        combined_log_writer.write_all(format!("{:?}\n", log).as_bytes()).await?;
    }
    writer.flush().await?;

    Ok(())
}
