use crate::driver::flash::SpiFlash;
use bytecheck::CheckBytes;
use defmt::*;
use rkyv::ser::{serializers::BufferSerializer, Serializer};
use rkyv::{AlignedBytes, Archive, Deserialize, Serialize};
use heapless::String;
use heapless::Vec;
use core::fmt::Write;

#[derive(Archive, Deserialize, Serialize, PartialEq, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub enum StorageMode {
    AVIONICS,
    GCM,
}

const STORAGE_VERSION: u32 = 0;
const STORAGE_META_ADDRESS: u32 = 0;

#[derive(Archive, Deserialize, Serialize, PartialEq, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub struct StorageMeta {
    storage_mode: StorageMode,
    storage_version: u32,
}

impl Default for StorageMeta {
    fn default() -> Self {
        Self {
            storage_mode: StorageMode::AVIONICS,
            storage_version: STORAGE_VERSION,
        }
    }
}

#[macro_export]
macro_rules! deserialize_safe {
    ($type:ident, $buffer:expr) => {{
        use rkyv::{check_archived_root, Infallible};
        use heapless::String;
        use core::fmt::Write;

        let buffer = $buffer;
        let result: Option<$type> = match check_archived_root::<$type>(buffer) {
            Ok(archived) => {
                let deserialized: $type = archived.deserialize(&mut Infallible).unwrap();
                Some(deserialized)
            }
            Err(e) => {
                let mut error_message = String::<128>::new();
                core::write!(&mut error_message, "{}", e).unwrap();
                warn!("Deserialization error: {:?}", error_message.as_str());
                None
            }
        };

        result
    }};
}

// pub async fn read_storage_meta<F: SpiFlash>(flash: &mut F) -> StorageMeta {
//     let buffer = flash.read_256_bytes(STORAGE_META_ADDRESS).await;

//     let deserialized = deserialize_safe!(StorageMeta, &buffer[8..]);

//     if let Some(storage_meta) = deserialized {
//         if storage_meta.storage_version == STORAGE_VERSION {
//             return storage_meta;
//         } else {
//             warn!(
//                 "Storage version mismatch, expected {}, got {}",
//                 STORAGE_VERSION, storage_meta.storage_version
//             );
//         }
//     }

//     warn!("Storage meta not found, using default");

//     let meta = StorageMeta::default();

//     write_storage_meta(flash, &meta).await;

//     meta
// }

// pub async fn write_storage_meta<F: SpiFlash>(flash: &mut F, meta: &StorageMeta) {
//     let mut serializer = BufferSerializer::new(AlignedBytes([0u8; 252]));
//     serializer
//         .serialize_value(meta)
//         .expect("failed to archive");
//     let length = serializer.pos() as u32;
//     let buf = serializer.into_inner();
//     let mut write_buffer = [0u8; 261];
//     (&mut write_buffer[5..9]).copy_from_slice(&length.to_be_bytes());
//     (&mut write_buffer[9..]).copy_from_slice(buf.as_ref());

//     flash.erase_sector_4kb(STORAGE_META_ADDRESS).await;
//     flash
//         .write_page(STORAGE_META_ADDRESS, &mut write_buffer)
//         .await;
// }
