#[macro_export]
macro_rules! create_serialized_logger {
    ($writer_struct_name:ident, $reader_struct_name:ident, $enum_name:ident, $buffer_size:expr, $(($log_type_i:expr, $log_type:ident)),*) => {
        #[derive(Debug)]
        enum $enum_name {
            $(
                $log_type($log_type),
            )*
        }

        struct $writer_struct_name<W: vlfs::AsyncWriter> {
            writer: W,
            buffer: [u8; $buffer_size],
        }

        impl<W: vlfs::AsyncWriter> $writer_struct_name<W> {
            pub fn new(writer: W) -> Self {
                Self {
                    writer,
                    buffer: [0; $buffer_size],
                }
            }

            pub async fn log(&mut self, log: $enum_name) -> Result<(), W::Error> {
                use rkyv::ser::Serializer;
                use rkyv::{ser::serializers::BufferSerializer};

                let mut serializer = BufferSerializer::new(self.buffer);
                match log {
                    $(
                        $enum_name::$log_type(log) => {
                            serializer.serialize_value(&log).unwrap();
                            let buffer = serializer.into_inner();
                            let buffer = &buffer[..core::mem::size_of::<<$log_type as Archive>::Archived>()];
                            self.writer.extend_from_u8($log_type_i).await?;
                            self.writer.extend_from_slice(buffer).await?;
                        }
                    )*
                };
                Ok(())
            }

            #[allow(dead_code)]
            pub fn into_writer(self) -> W {
                self.writer
            }
        }

        struct $reader_struct_name<R: vlfs::AsyncReader> {
            reader: R,
            buffer: [u8; 100],
        }

        impl<R: vlfs::AsyncReader> $reader_struct_name<R> {
            pub fn new(reader: R) -> Self {
                Self {
                    reader,
                    buffer: [0; 100],
                }
            }

            pub async fn next(&mut self) -> Result<Option<$enum_name>, R::Error> {
                use paste::paste;
                use rkyv::archived_root;
                let (typ, _) = self.reader.read_u8(&mut self.buffer).await?;
                match typ {
                    $(
                        Some($log_type_i) => {
                            let (slice,_) = self.reader.read_slice(&mut self.buffer, core::mem::size_of::<<$log_type as Archive>::Archived>()).await?;
                            let archived = unsafe { archived_root::<$log_type>(slice) };
                            let deserialized = <paste!{ [<Archived $log_type>] } as rkyv::Deserialize<$log_type, rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                            Ok(Some($enum_name::$log_type(deserialized)))
                        }
                    )*
                    Some(_) => {
                        Ok(None)
                    }
                    None=>{
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

#[cfg(test)]
mod file_logger_test {

    #[inline(never)]
    #[no_mangle]
    fn _defmt_acquire() {}
    #[inline(never)]
    #[no_mangle]
    fn _defmt_release() {}
    #[inline(never)]
    #[no_mangle]
    fn _defmt_flush() {}
    #[inline(never)]
    #[no_mangle]
    fn _defmt_write(_: &[u8]) {}
    #[inline(never)]
    #[no_mangle]
    fn _defmt_timestamp(_: defmt::Formatter<'_>) {}
    #[inline(never)]
    #[no_mangle]
    fn _defmt_panic() -> ! {
        loop {}
    }

    struct BufferWriter<'a> {
        pub buffer: &'a mut [u8; 4096],
        pub offset: usize,
    }

    impl<'a> vlfs::AsyncWriter for BufferWriter<'a> {
        type Error = ();
        async fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), Self::Error> {
            self.buffer[self.offset..self.offset + slice.len()].copy_from_slice(slice);
            self.offset += slice.len();
            Ok(())
        }
    }

    struct BufferReader<'b> {
        pub buffer: &'b mut [u8; 4096],
        pub offset: usize,
    }

    impl<'b> vlfs::AsyncReader for BufferReader<'b> {
        type Error = ();
        type ReadStatus = ();
        async fn read_slice<'a>(
            &mut self,
            buffer: &'a mut [u8],
            len: usize,
        ) -> Result<(&'a [u8], Self::ReadStatus), Self::Error> {
            (&mut buffer[0..len]).copy_from_slice(&self.buffer[self.offset..self.offset + len]);
            self.offset += len;
            Ok((&buffer[0..len], ()))
        }
    }

    use core::assert_matches::assert_matches;

    use rkyv::{Archive, Deserialize, Serialize};

    #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
    struct LogType1 {
        pub fielda: u32,
    }

    #[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
    struct LogType2 {
        pub fieldb: f32,
    }

    create_serialized_logger!(
        FileLogger, // this is the name of the struct
        FileLoggerReader,
        Log,
        100, // this is the buffer size
        (0, LogType1),
        (1, LogType2)
    );

    #[futures_test::test]
    async fn serialize_deserialize() {
        let mut buffer = [0u8; 4096];
        let writer = BufferWriter {
            buffer: &mut buffer,
            offset: 0,
        };

        let mut logger = FileLogger::new(writer);
        logger
            .log(Log::LogType1(LogType1 { fielda: 10 }))
            .await
            .unwrap();
        logger
            .log(Log::LogType2(LogType2 { fieldb: 1234.0 }))
            .await
            .unwrap();

        let offset = logger.into_writer().offset;
        println!("{:?}", &mut buffer[0..offset]);

        let reader = BufferReader {
            buffer: &mut buffer,
            offset: 0,
        };

        let mut logger_reader = FileLoggerReader::new(reader);
        assert_matches!(
            logger_reader.next().await,
            Ok(Some(Log::LogType1(LogType1 { fielda: 10 })))
        );
        assert_matches!(
            logger_reader.next().await,
            Ok(Some(Log::LogType2(LogType2 { fieldb: 1234.0 })))
        );
    }
}
