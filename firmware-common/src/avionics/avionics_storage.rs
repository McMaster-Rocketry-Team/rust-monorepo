use crate::deserialize_safe;
use crate::driver::flash::{SpiFlash, WriteBuffer};
use crate::storage::{StorageMeta, STORAGE_META_ADDRESS};
use bytecheck::CheckBytes;
use defmt::*;
use rkyv::ser::{serializers::BufferSerializer, Serializer};
use rkyv::{AlignedBytes, Archive, Deserialize, Serialize};

const AVIONICS_STORAGE_VERSION: u32 = 0;

#[derive(Archive, Deserialize, Serialize, Clone, defmt::Format)]
#[archive_attr(derive(CheckBytes))]
pub struct AvionicsStorageMeta {
    storage_version: u32,
}

impl Default for AvionicsStorageMeta {
    fn default() -> Self {
        Self {
            storage_version: AVIONICS_STORAGE_VERSION,
        }
    }
}

pub struct AvionicsStorage<F: SpiFlash> {
    flash: F,
    storage_meta: StorageMeta,
    avionics_meta: AvionicsStorageMeta,
}

impl<F: SpiFlash> AvionicsStorage<F> {
    pub async fn new(mut flash: F) -> Self {
        if let Some((storage_meta, avionics_meta)) = Self::try_read_meta(&mut flash).await {
            info!("Storage meta: {:?}", storage_meta);
            info!("Avionics storage meta: {:?}", avionics_meta);
            return Self {
                flash,
                storage_meta,
                avionics_meta,
            };
        } else {
            info!("Storage meta not found, initializing storage");
            let mut storage = Self {
                flash,
                storage_meta: StorageMeta::default(),
                avionics_meta: AvionicsStorageMeta::default(),
            };
            storage.write_meta().await;
            storage
        }
    }

    async fn try_read_meta(flash: &mut F) -> Option<(StorageMeta, AvionicsStorageMeta)> {
        let mut buffer = flash.read_256_bytes(STORAGE_META_ADDRESS).await;

        let length = buffer.read_u32() as usize;
        let storage_meta = deserialize_safe!(StorageMeta, buffer.read_slice(length));

        buffer.align_4_bytes();
        let length = buffer.read_u32() as usize;
        let avionics_storage_meta =
            deserialize_safe!(AvionicsStorageMeta, buffer.read_slice(length));

        if let (Some(storage_meta), Some(avionics_storage_meta)) =
            (storage_meta, avionics_storage_meta)
        {
            Some((storage_meta, avionics_storage_meta))
        } else {
            None
        }
    }

    pub async fn write_meta(&mut self) {
        self.flash.erase_sector_4kb(STORAGE_META_ADDRESS).await;

        let mut write_buffer = WriteBuffer::new();

        {
            let mut serializer = BufferSerializer::new(AlignedBytes([0u8; 252]));
            serializer
                .serialize_value(&self.storage_meta)
                .expect("failed to archive");
            let pos = serializer.pos() as u32;
            let buf = serializer.into_inner();

            write_buffer.extend_from_u32(pos);
            write_buffer.extend_from_slice(&buf[..pos as usize]);
        }

        {
            let mut serializer = BufferSerializer::new(AlignedBytes([0u8; 252]));
            serializer
                .serialize_value(&self.avionics_meta)
                .expect("failed to archive");
            let pos = serializer.pos() as u32;
            let buf = serializer.into_inner();

            write_buffer.align_4_bytes();
            write_buffer.extend_from_u32(pos);
            write_buffer.extend_from_slice(&buf[..pos as usize]);
        }

        self.flash.write_page(0, &mut write_buffer).await;
    }
}
