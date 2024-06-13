#![allow(warnings, unused)]

pub enum RpcClientError<S: crate::driver::serial::SplitableSerial> {
    Timeout,
    UnexpectedEof,
    Serial(S::Error),
}

impl<S: crate::driver::serial::SplitableSerial> From<embedded_io_async::ReadExactError<S::Error>>
    for RpcClientError<S>
{
    fn from(value: embedded_io_async::ReadExactError<S::Error>) -> Self {
        match value {
            embedded_io_async::ReadExactError::Other(e) => RpcClientError::Serial(e),
            embedded_io_async::ReadExactError::UnexpectedEof => RpcClientError::UnexpectedEof,
        }
    }
}

#[macro_export]
macro_rules! create_rpc {
    {
        $(enums {
            $(
                enum $enum_name:ident {
                    $( $enum_body:tt )*
                }
            )*
        })?
        $(rpc $rpc_i:literal $name:ident {
            request($($req_var_name:ident: $req_var_type:ty),*)
            response($($res_var_name:ident: $res_var_type:ty),*)
        })*
    } => {
        $(
            $(
                #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, defmt::Format)]
                pub enum $enum_name {
                    $( $enum_body )*
                }
            )*
        )?

        $(
            paste::paste! {
                #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, defmt::Format)]
                pub struct [< $name Request >] {
                    $(
                        pub $req_var_name: $req_var_type,
                    )*
                }
            }
            paste::paste! {
                #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, defmt::Format)]
                pub struct [< $name Response >] {
                    $(
                        pub $res_var_name: $res_var_type,
                    )*
                }
            }
        )*

        paste::paste! {
            // These two enums are only used to determine the max size of
            // the request and response during compile time
            #[allow(unused)]
            #[derive(rkyv::Archive)]
            enum RequestEnum {
                $(
                    [< $name Request >]([< $name Request >]),
                )*
            }

            #[allow(unused)]
            #[derive(rkyv::Archive)]
            enum ResponseEnum {
                $(
                    [< $name Response >]([< $name Response >]),
                )*
            }
        }

        paste::paste! {
            pub async fn run_rpc_server<S: crate::driver::serial::SplitableSerial, $([< F $rpc_i >]: futures::Future<Output = [< $name Response >]>,)*>(
                serial: &mut S,
                $(
                    mut [< $name:snake _handler >]: impl FnMut($($req_var_type ,)*) -> [< F $rpc_i >],
                )*
            ) -> Result<(), S::Error> {
                use core::mem::size_of;
                use rkyv::ser::Serializer;
                use rkyv::{ser::serializers::BufferSerializer};
                use rkyv::archived_root;
                use crc::{Crc, CRC_8_SMBUS};
                use embedded_io_async::Read;
                use embedded_io_async::ReadExactError;
                use embedded_io_async::Write;

                let crc = Crc::<u8>::new(&CRC_8_SMBUS);
                let mut request_buffer = [0u8; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1];
                let mut response_buffer = [0u8; size_of::<<ResponseEnum as rkyv::Archive>::Archived>()];
                let (mut tx, mut rx) = serial.split();

                loop {
                    match rx.read_exact(&mut request_buffer[..1]).await{
                        Ok(_) => {},
                        Err(ReadExactError::UnexpectedEof)=>{
                            continue;
                        },
                        Err(ReadExactError::Other(e))=>{
                            return Err(e);
                        }
                    }
                    match request_buffer[0] {
                        $(
                            $rpc_i => {
                                let request_size = size_of::<<[< $name Request >] as rkyv::Archive>::Archived>();
                                match rx.read_exact(&mut request_buffer[1..(request_size+2)]).await {
                                    Ok(_) => {},
                                    Err(ReadExactError::UnexpectedEof)=>{
                                        continue;
                                    },
                                    Err(ReadExactError::Other(e))=>{
                                        return Err(e);
                                    }
                                }

                                let calculated_crc = crc.checksum(&request_buffer[..(request_size+1)]);
                                let received_crc = request_buffer[request_size+1];
                                if calculated_crc != received_crc {
                                    defmt::info!("Command CRC mismatch, skipping.");
                                    continue;
                                }

                                let archived = unsafe { archived_root::<[< $name Request >]>(&request_buffer[1..(request_size+1)]) };
                                #[allow(unused)]
                                let deserialized = <[<Archived $name Request>] as rkyv::Deserialize<[< $name Request >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();

                                let response = [< $name:snake _handler >]($(deserialized.$req_var_name, )*).await;

                                let mut response_serializer = BufferSerializer::new(&mut response_buffer);
                                response_serializer.serialize_value(&response).unwrap();
                                drop(response_serializer);
                                let response_size = size_of::<<[< $name Response >] as rkyv::Archive>::Archived>();
                                response_buffer[response_size] = crc.checksum(&response_buffer[..response_size]);
                                tx.write_all(&response_buffer[..(response_size+1)]).await?;
                            }
                        )*
                        255 => {
                            tx.write_all(&[255, 0x69]).await?;
                        }
                        id => {
                            defmt::warn!("Unknown rpc id: {}", id);
                        }
                    }
                };
            }
        }

        paste::paste!{
            pub struct RpcClient<'a, S: crate::driver::serial::SplitableSerial, D: embedded_hal_async::delay::DelayNs> {
                serial: &'a mut S,
                delay: D,
            }

            impl<'a, S: crate::driver::serial::SplitableSerial, D: embedded_hal_async::delay::DelayNs> RpcClient<'a, S, D> {
                pub fn new(serial: &'a mut S, delay:D) -> Self {
                    Self { serial , delay}
                }

                async fn clear_read_buffer<T: embedded_io_async::Read>(delay: &mut D, rx: &mut T) {
                    use crate::utils::run_with_timeout;
                    let read_fut = async {
                        let mut buffer = [0u8;64];
                        loop {
                            let result = rx.read(&mut buffer).await;
                            if let Ok(0) = result {
                                break;
                            }
                        }
                    };
                    run_with_timeout(delay,100.0,read_fut).await.ok();
                }

                pub async fn reset(&mut self) ->  Result<bool, crate::common::console::create_rpc::RpcClientError<S>> {
                    use embedded_io_async::Write;
                    use embedded_io_async::Read;
                    use crate::common::console::create_rpc::RpcClientError;
                    use core::mem::size_of;

                    let (mut tx, mut rx) = self.serial.split();

                    // flush the serial buffer
                    tx.write_all(&[255; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1]).await.map_err(RpcClientError::Serial)?;
                    Self::clear_read_buffer(&mut self.delay, &mut rx).await;

                    // send reset command
                    tx.write_all(&[255]).await.map_err(RpcClientError::Serial)?;

                    let mut buffer = [0u8; 2];
                    rx.read_exact(&mut buffer).await?;
                    if buffer == [255, 0x69] {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }

                $(
                    pub async fn [< $name:snake >](&mut self, $($req_var_name: $req_var_type, )*) -> Result<[< $name Response >], crate::common::console::create_rpc::RpcClientError<S>> {
                        use core::mem::size_of;
                        use rkyv::ser::Serializer;
                        use rkyv::{ser::serializers::BufferSerializer};
                        use rkyv::archived_root;
                        use embedded_io_async::Write;
                        use embedded_io_async::Read;
                        use crate::common::console::create_rpc::RpcClientError;

                        let (mut tx, mut rx) = self.serial.split();

                        let mut request_buffer = [0u8; size_of::<<[< $name Request >] as rkyv::Archive>::Archived>()];
                        let mut request_serializer = BufferSerializer::new(&mut request_buffer);
                        let request = [< $name Request >] {
                            $(
                                $req_var_name,
                            )*
                        };
                        request_serializer.serialize_value(&request).unwrap();
                        drop(request_serializer);
                        tx.write_all(&request_buffer).await.map_err(RpcClientError::Serial)?;

                        let mut response_buffer = [0u8; size_of::<<[< $name Response >] as rkyv::Archive>::Archived>()];
                        // TODO timeout
                        rx.read_exact(&mut response_buffer).await?;
                        let archived = unsafe { archived_root::<[< $name Response >]>(&response_buffer) };
                        let deserialized = <[<Archived $name Response>] as rkyv::Deserialize<[< $name Response >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                        Ok(deserialized)
                    }
                )*

            }
        }
    };
}
