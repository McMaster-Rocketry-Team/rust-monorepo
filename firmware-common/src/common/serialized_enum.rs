use core::fmt::Debug;

use embedded_io_async::{Read, ReadExactError};

pub trait SerializedEnumReader<R: Read> {
    type Output: Debug;

    fn new(reader: R) -> Self;

    async fn read_next(
        &mut self,
    ) -> Result<Option<Self::Output>, ReadExactError<R::Error>>;

    fn into_reader(self) -> R;
}

#[macro_export]
macro_rules! create_serialized_enum {
    ($writer_struct_name:ident, $reader_struct_name:ident, $enum_name:ident, $(($log_type_i:expr, $log_type:ident)),*) => {
        #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Clone, defmt::Format)]
        pub enum $enum_name {
            $(
                $log_type($log_type),
            )*
        }

        $(
            impl From<$log_type> for $enum_name {
                fn from(log: $log_type) -> Self {
                    $enum_name::$log_type(log)
                }
            }
        )*

        impl $enum_name {
            pub fn write_to_buffer(&self, buffer: &mut [u8]) -> usize {
                use rkyv::ser::Serializer;
                use rkyv::{ser::serializers::BufferSerializer};
                use core::mem::size_of;

                match self {
                    $(
                        $enum_name::$log_type(log) => {
                            buffer[0] = $log_type_i;
                            let mut serializer = BufferSerializer::new(&mut buffer[1..]);
                            serializer.serialize_value(log).unwrap();
                            return size_of::<<$log_type as rkyv::Archive>::Archived>() + 1;
                        }
                    )*
                };
            }

            pub fn from_buffer(buffer: &[u8]) -> Option<Self> {
                use rkyv::{archived_root, AlignedBytes};
                use core::mem::size_of;
                use paste::paste;

                match buffer[0] {
                    $(
                        $log_type_i => {
                            let size = size_of::<<$log_type as rkyv::Archive>::Archived>();
                            if buffer.len() < size+1 {
                                return None;
                            }
                            let mut aligned_buffer: AlignedBytes<{ size_of::<<$log_type as rkyv::Archive>::Archived>() }> = Default::default();
                            aligned_buffer.as_mut().copy_from_slice(&buffer[1..size+1]);
                            let archived = unsafe { archived_root::<$log_type>(aligned_buffer.as_ref()) };
                            let deserialized = <paste!{ [<Archived $log_type>] } as rkyv::Deserialize<$log_type, rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                            Some($enum_name::$log_type(deserialized))
                        }
                    )*
                    _ => {
                        None
                    }
                }
            }
        }

        pub struct $writer_struct_name<W: embedded_io_async::Write> {
            writer: W,
            buffer: [u8; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
        }

        impl<W: embedded_io_async::Write> $writer_struct_name<W> {
            pub fn new(writer: W) -> Self {
                Self {
                    writer,
                    buffer: [0; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
                }
            }

            pub async fn write(&mut self, log: &$enum_name) -> Result<(), W::Error> {
                let len = log.write_to_buffer(&mut self.buffer);
                self.writer.write_all(&self.buffer[..len]).await?;
                Ok(())
            }

            pub async fn flush(&mut self) -> Result<(), W::Error> {
                self.writer.flush().await
            }

            #[allow(dead_code)]
            pub fn into_writer(self) -> W {
                self.writer
            }
        }

        #[repr(C, align(16))]
        pub struct $reader_struct_name<R: embedded_io_async::Read> {
            buffer: [u8; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
            reader: R,
        }

        impl<R: embedded_io_async::Read> crate::common::serialized_enum::SerializedEnumReader<R> for $reader_struct_name<R> {
            type Output = $enum_name;

            fn new(reader: R) -> Self {
                Self {
                    reader,
                    buffer: [0; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
                }
            }

            async fn read_next(&mut self) -> Result<Option<$enum_name>, embedded_io_async::ReadExactError<R::Error>> {
                use paste::paste;
                use rkyv::archived_root;
                self.reader.read_exact(&mut self.buffer[..1]).await?;
                match self.buffer[0] {
                    $(
                        $log_type_i => {
                            let size = core::mem::size_of::<<$log_type as rkyv::Archive>::Archived>();
                            self.reader.read_exact(&mut self.buffer[..size]).await?;
                            let archived = unsafe { archived_root::<$log_type>(&self.buffer[..size]) };
                            let deserialized = <paste!{ [<Archived $log_type>] } as rkyv::Deserialize<$log_type, rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                            Ok(Some($enum_name::$log_type(deserialized)))
                        }
                    )*
                    _ => {
                        Ok(None)
                    }
                }
            }

            #[allow(dead_code)]
            fn into_reader(self) -> R {
                self.reader
            }
        }
    };
}

#[cfg(test)]
mod file_logger_test {
    use crate::common::{serialized_enum::SerializedEnumReader, test_utils::BufferWriter};
    use core::assert_matches::assert_matches;
    use rkyv::{Archive, Deserialize, Serialize};

    #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
    pub struct LogType1 {
        pub fielda: u32,
    }

    #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
    pub struct LogType2 {
        pub fieldb: f32,
    }

    create_serialized_enum!(
        FileLogger, // this is the name of the struct
        FileLoggerReader,
        Log,
        (0, LogType1),
        (1, LogType2)
    );

    #[futures_test::test]
    async fn serialize_deserialize() {
        let mut buffer = [0u8; 4096];
        let writer = BufferWriter::new(&mut buffer);

        let mut logger = FileLogger::new(writer);
        logger
            .write(&Log::LogType1(LogType1 { fielda: 10 }))
            .await
            .unwrap();
        logger
            .write(&Log::LogType2(LogType2 { fieldb: 1234.0 }))
            .await
            .unwrap();

        let reader = logger.into_writer().into_reader();

        let mut logger_reader = FileLoggerReader::new(reader);
        assert_matches!(
            logger_reader.read_next().await,
            Ok(Some(Log::LogType1(LogType1 { fielda: 10 })))
        );
        assert_matches!(
            logger_reader.read_next().await,
            Ok(Some(Log::LogType2(LogType2 { fieldb: 1234.0 })))
        );
    }
}
