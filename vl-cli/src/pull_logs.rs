use firmware_common::{
    common::serialized_enum::SerializedEnumReader, driver::serial::SplitableSerial, CommonRPCTrait,
};
use std::path::PathBuf;
use tokio::{fs::File, io::{AsyncWriteExt, BufReader, BufWriter}};

use anyhow::Result;

use crate::{pull_file::pull_file, reader::BufReaderWrapper, PullArgs};

pub async fn pull_logs<S: SplitableSerial,SR: SerializedEnumReader<BufReaderWrapper<File>>>(
    rpc: &mut impl CommonRPCTrait<S>,
    save_folder: PathBuf,
    file_id: u64,
    file_type_name: &str,
    combined_log_writer: &mut BufWriter<File>
) -> Result<()> {
    // VLE: void lake enums
    let mut vle_path = save_folder.clone();
    vle_path.push(format!("{}.{}.vle", file_id, file_type_name));
    pull_file(
        rpc,
        PullArgs {
            file_id,
            host_path: vle_path.clone(),
        },
    )
    .await?;

    let mut log_path = save_folder.clone();
    log_path.push(format!("{}.{}.log", file_id, file_type_name));
    let reader = BufReader::new(File::open(vle_path).await?);
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
