use rkyv::{Archive, Deserialize, Serialize};

pub mod create_rpc;
pub mod vl_rpc;
pub mod sg_rpc;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, defmt::Format)]
pub enum DeviceType {
    VoidLake,
    OZYS,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, defmt::Format)]
enum OpenFileStatus {
    Sucess,
    DoesNotExist,
    Error,
}