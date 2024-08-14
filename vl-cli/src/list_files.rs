use crate::LSArgs;
use anyhow::Result;
use firmware_common::{driver::serial::SplitableSerial, CommonRPCTrait};
use vlfs::{FileID, FileType};

pub async fn list_files<S: SplitableSerial>(
    rpc: &mut impl CommonRPCTrait<S>,
    args: LSArgs,
) -> Result<Vec<FileID>> {
    let mut result = Vec::new();

    rpc.start_list_files(args.file_type.map(FileType)).await.unwrap();
    loop {
        let response = rpc.get_listed_file().await.unwrap();
        if let Some(file_id) = response {
            result.push(file_id);
        } else {
            break;
        }
    }

    Ok(result)
}
