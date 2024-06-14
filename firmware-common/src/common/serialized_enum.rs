#[macro_export]
macro_rules! create_serialized_enum {
    ($writer_struct_name:ident, $reader_struct_name:ident, $enum_name:ident, $(($log_type_i:expr, $log_type:ident)),*) => {
        #[derive(rkyv::Archive, Debug, Clone, defmt::Format)]
        pub enum $enum_name {
            $(
                $log_type($log_type),
            )*
        }

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
                            return size_of::<<$log_type as Archive>::Archived>() + 1;
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
                            let size = size_of::<<$log_type as Archive>::Archived>();
                            if buffer.len() < size+1 {
                                return None;
                            }
                            let mut aligned_buffer: AlignedBytes<{ size_of::<<$log_type as Archive>::Archived>() }> = Default::default();
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

        struct $writer_struct_name<W: embedded_io_async::Write> {
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

            #[allow(dead_code)]
            pub fn into_writer(self) -> W {
                self.writer
            }
        }

        #[repr(C, align(16))]
        struct $reader_struct_name<R: embedded_io_async::Read> {
            buffer: [u8; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
            reader: R,
        }

        impl<R: embedded_io_async::Read> $reader_struct_name<R> {
            pub fn new(reader: R) -> Self {
                Self {
                    reader,
                    buffer: [0; core::mem::size_of::<<$enum_name as rkyv::Archive>::Archived>()],
                }
            }

            pub async fn read_next(&mut self) -> Result<Option<$enum_name>, embedded_io_async::ReadExactError<R::Error>> {
                use paste::paste;
                use rkyv::archived_root;
                self.reader.read_exact(&mut self.buffer[..1]).await?;
                match self.buffer[0] {
                    $(
                        $log_type_i => {
                            let size = core::mem::size_of::<<$log_type as Archive>::Archived>();
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
            pub fn into_reader(self) -> R {
                self.reader
            }
        }
    };
}

// #[cfg(test)]
// mod file_logger_test {

//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_acquire() {}
//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_release() {}
//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_flush() {}
//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_write(_: &[u8]) {}
//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_timestamp(_: defmt::Formatter<'_>) {}
//     #[inline(never)]
//     #[no_mangle]
//     fn _defmt_panic() -> ! {
//         loop {}
//     }

//     struct BufferWriter<'a> {
//         pub buffer: &'a mut [u8; 4096],
//         pub offset: usize,
//     }

//     impl<'a> vlfs::AsyncWriter for BufferWriter<'a> {
//         type Error = ();
//         async fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), Self::Error> {
//             self.buffer[self.offset..self.offset + slice.len()].copy_from_slice(slice);
//             self.offset += slice.len();
//             Ok(())
//         }
//     }

//     struct BufferReader<'b> {
//         pub buffer: &'b mut [u8; 4096],
//         pub offset: usize,
//     }

//     impl<'b> vlfs::AsyncReader for BufferReader<'b> {
//         type Error = ();
//         type ReadStatus = ();
//         async fn read_slice<'a>(
//             &mut self,
//             buffer: &'a mut [u8],
//             len: usize,
//         ) -> Result<(&'a [u8], Self::ReadStatus), Self::Error> {
//             (&mut buffer[0..len]).copy_from_slice(&self.buffer[self.offset..self.offset + len]);
//             self.offset += len;
//             Ok((&buffer[0..len], ()))
//         }
//     }

//     use core::assert_matches::assert_matches;

//     use rkyv::{Archive, Deserialize, Serialize};

//     #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
//     struct LogType1 {
//         pub fielda: u32,
//     }

//     #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
//     struct LogType2 {
//         pub fieldb: f32,
//     }

//     create_serialized_enum!(
//         FileLogger, // this is the name of the struct
//         FileLoggerReader,
//         Log,
//         (0, LogType1),
//         (1, LogType2)
//     );

//     #[futures_test::test]
//     async fn serialize_deserialize() {
//         let mut buffer = [0u8; 4096];
//         let writer = BufferWriter {
//             buffer: &mut buffer,
//             offset: 0,
//         };

//         let mut logger = FileLogger::new(writer);
//         logger
//             .log(Log::LogType1(LogType1 { fielda: 10 }))
//             .await
//             .unwrap();
//         logger
//             .log(Log::LogType2(LogType2 { fieldb: 1234.0 }))
//             .await
//             .unwrap();

//         let offset = logger.into_writer().offset;
//         println!("{:?}", &mut buffer[0..offset]);

//         let reader = BufferReader {
//             buffer: &mut buffer,
//             offset: 0,
//         };

//         let mut logger_reader = FileLoggerReader::new(reader);
//         assert_matches!(
//             logger_reader.next().await,
//             Ok(Some(Log::LogType1(LogType1 { fielda: 10 })))
//         );
//         assert_matches!(
//             logger_reader.next().await,
//             Ok(Some(Log::LogType2(LogType2 { fieldb: 1234.0 })))
//         );
//     }
// }
