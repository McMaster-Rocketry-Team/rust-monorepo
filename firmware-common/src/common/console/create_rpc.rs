#![allow(warnings, unused)]

use crate::driver::serial::DummySerial;

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
            pub async fn run_rpc_server<S: crate::driver::serial::Serial, $([< F $rpc_i >]: futures::Future<Output = ([< $name Response >], bool)>,)*>(
                serial: &mut S,
                $(
                    mut [< $name:snake _handler >]: impl FnMut($($req_var_type ,)*) -> [< F $rpc_i >],
                )*
            ) -> Result<(), S::Error> {
                use core::mem::size_of;
                use rkyv::ser::Serializer;
                use rkyv::{ser::serializers::BufferSerializer};
                use rkyv::archived_root;

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

                let mut request_buffer = [0u8; size_of::<<RequestEnum as rkyv::Archive>::Archived>()];
                let mut response_buffer = [0u8; size_of::<<ResponseEnum as rkyv::Archive>::Archived>()];

                loop {
                    serial.read_all(&mut request_buffer[..1]).await?;
                    match request_buffer[0] {
                        $(
                            $rpc_i => {
                                let request_size = size_of::<<[< $name Request >] as rkyv::Archive>::Archived>();
                                serial.read_all(&mut request_buffer[..request_size]).await?;
                                let archived = unsafe { archived_root::<[< $name Request >]>(&request_buffer[..request_size]) };
                                #[allow(unused)]
                                let deserialized = <[<Archived $name Request>] as rkyv::Deserialize<[< $name Request >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();

                                let (response, should_end) = [< $name:snake _handler >]($(deserialized.$req_var_name, )*).await;

                                let mut response_serializer = BufferSerializer::new(&mut response_buffer);
                                response_serializer.serialize_value(&response).unwrap();
                                drop(response_serializer);
                                serial.write(&response_buffer[..size_of::<<[< $name Response >] as rkyv::Archive>::Archived>()]).await?;
                            
                                if should_end {
                                    break;
                                }
                            }
                        )*
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
            }
        }
    };
}

create_rpc! {
    enums {
        enum OpenFileStatus {
            Sucess,
            DoesNotExist,
            Error,
        }
    }
    rpc 0 OpenFile {
        request(file_id: u64, file_type: u32)
        response(status: OpenFileStatus)
    }
    rpc 1 CloseFile {
        request()
        response()
    }
}

async fn test(mut serial: impl crate::driver::serial::Serial) {
    run_rpc_server(
        &mut serial,
        async |file_id, file_type| {
            (
                OpenFileResponse {
                    status: OpenFileStatus::Error,
                },
                false,
            )
        },
        async || (CloseFileResponse {}, true),
    )
    .await;
}

fn execute_async_closure<Fut>(closure: impl Fn() -> Fut)
where
    Fut: futures::Future<Output = ()>,
{
    // Example of calling the closure multiple times
    async {
        closure().await;
        closure().await;
    };
}

