use rkyv::{Archive, Deserialize, Serialize};

pub mod create_rpc;
pub mod common_rpc_trait;
pub mod vl_rpc;
pub mod sg_rpc;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, defmt::Format)]
pub enum DeviceType {
    VoidLake,
    OZYS,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, defmt::Format)]
pub enum OpenFileStatus {
    Sucess,
    DoesNotExist,
    Error,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, defmt::Format)]
pub struct ReadFileResult {
    pub data: [u8; 128],
    pub length: u8,
    pub corrupted: bool,
}