#![allow(warnings, unused)]

use crc::{Crc, CRC_8_SMBUS};

fn test() {
    let crc = Crc::<u8>::new(&CRC_8_SMBUS);
    let a = crc.checksum(b"awa");
}

#[macro_export]
macro_rules! create_rpc {
    {
        $(enums {
            $(
                enum $enum_name:ident $( $enum_body:tt )*
            )*
        })?
        $(rpc $rpc_i:literal $name:ident {
            request($($req_var_name:ident: $req_var_type:ty),*)
            response($($res_var_name:ident: $res_var_type:ty),*)
        })*
    } => {
        $(
            #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, defmt::Format)]
            $(
                pub enum $enum_name $( $enum_body )*
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
            pub async fn run_rpc_server<ST: embedded_io_async::Write, SR: embedded_io_async::Read, $([< F $rpc_i >]: futures::Future<Output = [< $name Response >]>,)* FC: futures::Future<Output = ()>>(
                serial: &mut (ST, SR),
                $(
                    mut [< $name:snake _handler >]: impl FnMut($($req_var_type ,)*) -> [< F $rpc_i >],
                )*
                mut close_handler: impl FnMut()-> FC
            ) -> Result<(), S::Error> {
                use core::mem::size_of;
                use rkyv::ser::Serializer;
                use rkyv::{ser::serializers::BufferSerializer};
                use rkyv::archived_root;
                use crc::{Crc, CRC_8_SMBUS};



                let crc = Crc::<u8>::new(&CRC_8_SMBUS);
                let mut request_buffer = [0u8; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1];
                let mut response_buffer = [0u8; size_of::<<ResponseEnum as rkyv::Archive>::Archived>()];

                loop {
                    serial.read_all(&mut request_buffer[..1]).await?;
                    match request_buffer[0] {
                        $(
                            $rpc_i => {
                                let request_size = size_of::<<[< $name Request >] as rkyv::Archive>::Archived>();
                                serial.read_all(&mut request_buffer[1..(request_size+2)]).await?;

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
                                serial.write(&response_buffer[..(response_size+1)]).await?;
                            }
                        )*
                        255 => {
                            close_handler().await;
                            serial.write(&[255, 0x69]).await?;
                            break;
                        }
                        id => {
                            defmt::warn!("Unknown rpc id: {}", id);
                        }
                    }
                };
                Ok(())
            }
        }

        paste::paste!{
            pub struct RpcClient<'a, S: crate::driver::serial::Serial> {
                serial: &'a mut S,
            }

            impl<'a, S: crate::driver::serial::Serial> RpcClient<'a, S> {
                pub fn new(serial: &'a mut S) -> Self {
                    Self { serial }
                }

                pub async fn reset(&mut self) ->  Result<bool, S::Error> {
                    // flush the serial buffer
                    self.serial.write(&[255; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1]).await?;
                    let mut buffer = [0u8; 2];
                    todo!()
                }

                $(
                    pub async fn [< $name:snake >](&mut self, $($req_var_name: $req_var_type, )*) -> Result<[< $name Response >], S::Error> {
                        use core::mem::size_of;
                        use rkyv::ser::Serializer;
                        use rkyv::{ser::serializers::BufferSerializer};
                        use rkyv::archived_root;

                        let mut request_buffer = [0u8; size_of::<<[< $name Request >] as rkyv::Archive>::Archived>()];
                        let mut request_serializer = BufferSerializer::new(&mut request_buffer);
                        let request = [< $name Request >] {
                            $(
                                $req_var_name,
                            )*
                        };
                        request_serializer.serialize_value(&request).unwrap();
                        drop(request_serializer);
                        self.serial.write(&request_buffer).await?;

                        let mut response_buffer = [0u8; size_of::<<[< $name Response >] as rkyv::Archive>::Archived>()];
                        self.serial.read_all(&mut response_buffer).await?;
                        let archived = unsafe { archived_root::<[< $name Response >]>(&response_buffer) };
                        let deserialized = <[<Archived $name Response>] as rkyv::Deserialize<[< $name Response >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                        Ok(deserialized)
                    }
                )*

                pub async fn close(&mut self) -> Result<(), S::Error> {
                    self.serial.write(&[255]).await?;
                    let mut buffer = [0u8; 1];
                    self.serial.read_all(&mut buffer).await?;
                    assert_eq!(buffer[0], 255);
                    Ok(())
                }
            }
        }
    };
}
