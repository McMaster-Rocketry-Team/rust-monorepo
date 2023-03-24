use crate::deserialize_safe;
use crate::driver::flash::{SpiFlash, WriteBuffer};
use crate::common::storage_meta::{StorageMeta, STORAGE_META_ADDRESS};
use bytecheck::CheckBytes;
use defmt::*;
use rkyv::ser::{serializers::BufferSerializer, Serializer};
use rkyv::{AlignedBytes, Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub struct GCMStorageMeta {
    name: [u8; 64],
    rocket_model_size: u32, // bytes
}

impl Default for GCMStorageMeta {
    fn default() -> Self {
        Self {
            rocket_model_size: 0,
            name: [0; 64],
        }
    }
}

pub struct GCMStorage<F: SpiFlash> {
    flash: F,
    storage_meta: StorageMeta,
    gcm_meta: GCMStorageMeta,
}