use crate::{
    avionics::avionics_storage::AvionicsStorage,
    deserialize_safe,
    driver::flash::{ReadBuffer, SpiFlash},
    gcm::gcm_storage::GCMStorage,
};

use super::storage_meta::{StorageMeta, STORAGE_META_ADDRESS, STORAGE_VERSION};

// pub async fn get_storage<F: SpiFlash>(
//     mut flash: F,
// ) -> (Option<AvionicsStorage<F>>, Option<GCMStorage<F>>) {
//     let mut buffer = flash.read_256_bytes(STORAGE_META_ADDRESS).await;

//     let storage_meta = try_read_storage_meta(&mut buffer).await;
//     if let Some(storage_meta) = storage_meta {
//         let mode = storage_meta.storage_mode;
//     }

//     (None, None)
// }

// async fn try_read_storage_meta(buffer: &mut ReadBuffer) -> Option<StorageMeta> {
//     let length = buffer.read_u32() as usize;
//     let storage_meta = deserialize_safe!(StorageMeta, buffer.read_slice(length));

//     if let Some(storage_meta) = storage_meta {
//         if storage_meta.storage_version != STORAGE_VERSION {
//             None
//         } else {
//             Some(storage_meta)
//         }
//     } else {
//         None
//     }
// }
