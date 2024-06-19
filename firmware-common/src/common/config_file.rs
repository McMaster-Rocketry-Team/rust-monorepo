use core::marker::PhantomData;
use core::mem::size_of;

use embedded_io_async::Read;
use rkyv::{
    archived_root,
    ser::{serializers::BufferSerializer, Serializer},
    AlignedBytes, Archive, Deserialize, Serialize,
};
use vlfs::{AsyncWriter, Crc, FileType, Flash, VLFSError, VLFS};

use crate::avionics::flight_profile::FlightProfile;

use super::{device_config::DeviceConfig, file_types::{DEVICE_CONFIG_FILE_TYPE, FLIGHT_PROFILE_FILE_TYPE}};

pub struct ConfigFile<'a, T, F, C>
where
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    F: Flash,
    C: Crc,
    [(); size_of::<T::Archived>()]:,
{
    _phantom: PhantomData<T>,
    file_type: FileType,
    fs: &'a VLFS<F, C>,
}

impl<'a, T, F, C> ConfigFile<'a, T, F, C>
where
    T: Archive + Serialize<BufferSerializer<[u8; size_of::<T::Archived>()]>>,
    T::Archived: Deserialize<T, rkyv::Infallible>,
    F: Flash,
    C: Crc,
    [(); size_of::<T::Archived>()]:,
{
    pub fn new(fs: &'a VLFS<F, C>, file_type: FileType) -> Self {
        ConfigFile {
            _phantom: PhantomData,
            fs,
            file_type,
        }
    }

    pub async fn read(&self) -> Option<T> {
        if let Ok(Some(file)) = self.fs.find_first_file_by_type(self.file_type).await {
            match self.fs.open_file_for_read(file.id).await {
                Ok(mut reader) => {
                    let mut buffer: AlignedBytes<{ size_of::<T::Archived>() }> = Default::default();
                    // buffer.as_mut().copy_from_slice(src)
                    let result = reader.read_exact(buffer.as_mut()).await;
                    reader.close().await;
                    if let Err(e) = result {
                        log_warn!("Failed to read config file {:?}: {:?}", self.file_type, e);
                        return None;
                    }

                    let archived = unsafe { archived_root::<T>(buffer.as_ref()) };
                    let deserialized =
                        <T::Archived as rkyv::Deserialize<T, rkyv::Infallible>>::deserialize(
                            archived,
                            &mut rkyv::Infallible,
                        )
                        .unwrap();
                    return Some(deserialized);
                }
                Err(e) => {
                    log_warn!("Failed to open config file {:?}: {:?}", self.file_type, e);
                    return None;
                }
            }
        } else {
            log_info!("Config file {:?} not found", self.file_type);
            return None;
        }
    }

    pub async fn write(&self, config: &T) -> Result<(), VLFSError<F::Error>> {
        self.fs.remove_files_with_type(self.file_type).await?;

        let file = self.fs.create_file(self.file_type).await?;
        let mut writer = self.fs.open_file_for_write(file.id).await?;

        let buffer = [0u8; size_of::<T::Archived>()];
        let mut serializer = BufferSerializer::new(buffer);
        serializer.serialize_value(config).unwrap();
        let buffer = serializer.into_inner();

        writer.extend_from_slice(&buffer).await?;
        todo!()
    }
}
