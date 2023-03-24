use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub enum StorageMode {
    AVIONICS,
    GCM,
}

pub const STORAGE_VERSION: u32 = 1;
pub const STORAGE_META_ADDRESS: u32 = 0;

#[derive(Archive, Deserialize, Serialize, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub struct StorageMeta {
    pub storage_mode: StorageMode,
    pub storage_version: u32,
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
        use rkyv::{check_archived_root, Infallible, Deserialize};
        use heapless::String;
        use core::fmt::Write;
        use defmt::*;

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

