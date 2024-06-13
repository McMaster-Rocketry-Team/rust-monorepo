#![allow(warnings, unused)]

#[derive(defmt::Format, core::fmt::Debug)]
pub enum RpcClientError {
    Timeout,
    ECCMismatch,
    UnexpectedEof,
    Serial,
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
        state<$($generics_name:ident: $generics_type:ty),*>($($para_name:ident: $para_type:ty),*) {
            $( $state_body:tt )*
        }
        $(rpc $rpc_i:literal $name:ident |$($req_var_name:ident: $req_var_type:ty),*| -> ($($res_var_name:ident: $res_var_type:ty),*) $handler:expr)*
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
            pub async fn run_rpc_server<
                S: crate::driver::serial::SplitableSerial, 
                $(
                    $generics_name: $generics_type,
                )*
            >(
                serial: &mut S,
                $(
                    $para_name: $para_type,
                )*
            ) {
                let result: Result<(), S::Error> = try {
                    use core::mem::size_of;
                    use rkyv::ser::Serializer;
                    use rkyv::{ser::serializers::BufferSerializer};
                    use rkyv::archived_root;
                    use crc::{Crc, CRC_8_SMBUS};
                    use embedded_io_async::Read;
                    use embedded_io_async::ReadExactError;
                    use embedded_io_async::Write;

                    $( $state_body )*

                    let crc = Crc::<u8>::new(&CRC_8_SMBUS);
                    let mut request_buffer = [0u8; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1];
                    let mut response_buffer = [0u8; size_of::<<ResponseEnum as rkyv::Archive>::Archived>()];
                    let (mut tx, mut rx) = serial.split();

                    loop {
                        match rx.read_exact(&mut request_buffer[..1]).await{
                            Ok(_) => {},
                            Err(ReadExactError::UnexpectedEof)=>{
                                log_info!("Unexpected EOF, skipping.");
                                continue;
                            },
                            Err(ReadExactError::Other(e))=>{
                                Err(e)?;
                            }
                        }
                        log_info!("Received command: {:x}", request_buffer[0]);
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
                                            Err(e)?;
                                        }
                                    }

                                    let calculated_crc = crc.checksum(&request_buffer[..(request_size+1)]);
                                    let received_crc = request_buffer[request_size+1];
                                    if calculated_crc != received_crc {
                                        log_info!("Command CRC mismatch, skipping.");
                                        continue;
                                    }
                                    log_info!("Command CRC matched.");

                                    let archived = unsafe { archived_root::<[< $name Request >]>(&request_buffer[1..(request_size+1)]) };
                                    #[allow(unused)]
                                    let request = <[<Archived $name Request>] as rkyv::Deserialize<[< $name Request >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();

                                    $(
                                        let $req_var_name = request.$req_var_name;
                                    )*
                                    let response = $handler;

                                    

                                    let mut response_serializer = BufferSerializer::new(&mut response_buffer);
                                    response_serializer.serialize_value(&response).unwrap();
                                    drop(response_serializer);
                                    let response_size = size_of::<<[< $name Response >] as rkyv::Archive>::Archived>();
                                    response_buffer[response_size] = crc.checksum(&response_buffer[..response_size]);
                                    tx.write_all(&response_buffer[..(response_size+1)]).await?;
                                    log_info!("Response sent.");
                                }
                            )*
                            255 => {
                                tx.write_all(&[255, 0x69]).await?;
                            }
                            id => {
                                log_warn!("Unknown rpc id: {}", id);
                            }
                        }
                    };
                };
                if let Err(e) = result {
                    log_error!("Error running console: {:?}", e);
                }
            }
        }

        paste::paste!{
            pub struct RpcClient<'a, S: crate::driver::serial::SplitableSerial, D: embedded_hal_async::delay::DelayNs> {
                serial: &'a mut S,
                delay: D,
            }

            impl<'a, S: crate::driver::serial::SplitableSerial, D: embedded_hal_async::delay::DelayNs> RpcClient<'a, S, D> {
                pub fn new(serial: &'a mut S, delay: D) -> Self {
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
                    run_with_timeout(delay, 100.0, read_fut).await.ok();
                }

                pub async fn reset(&mut self) ->  Result<bool, crate::common::console::create_rpc::RpcClientError> {
                    use embedded_io_async::Write;
                    use embedded_io_async::Read;
                    use crate::common::console::create_rpc::RpcClientError;
                    use core::mem::size_of;
                    
                    let (mut tx, mut rx) = self.serial.split();

                    // flush the serial buffer
                    tx.write_all(&[255; size_of::<<RequestEnum as rkyv::Archive>::Archived>() + 1]).await.map_err(|_|RpcClientError::Serial)?;
                    Self::clear_read_buffer(&mut self.delay, &mut rx).await;

                    // send reset command
                    tx.write_all(&[255]).await.map_err(|_|RpcClientError::Serial)?;

                    let mut buffer = [0u8; 2];
                    rx.read_exact(&mut buffer).await.map_err(|_|RpcClientError::Serial)?;
                    if buffer == [255, 0x69] {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }

                $(
                    pub async fn [< $name:snake >](&mut self, $($req_var_name: $req_var_type, )*) -> Result<[< $name Response >], crate::common::console::create_rpc::RpcClientError> {
                        use core::mem::size_of;
                        use rkyv::ser::Serializer;
                        use rkyv::{ser::serializers::BufferSerializer};
                        use rkyv::archived_root;
                        use embedded_io_async::Write;
                        use embedded_io_async::Read;
                        use crate::common::console::create_rpc::RpcClientError;
                        use crc::{Crc, CRC_8_SMBUS};
                        use crate::utils::run_with_timeout;
                        use futures::join;

                        let crc = Crc::<u8>::new(&CRC_8_SMBUS);
                        let (mut tx, mut rx) = self.serial.split();

                        let mut request_buffer = [0u8; size_of::<<[< $name Request >] as rkyv::Archive>::Archived>() + 1];
                        request_buffer[0] = $rpc_i;
                        let mut request_serializer = BufferSerializer::new(&mut request_buffer[1..]);
                        let request = [< $name Request >] {
                            $(
                                $req_var_name,
                            )*
                        };
                        request_serializer.serialize_value(&request).unwrap();
                        drop(request_serializer);

                        let tx_fut = async {
                            let result: Result<(), RpcClientError> = try {
                                tx.write_all(&request_buffer).await.map_err(|_|RpcClientError::Serial)?;
                                tx.write_all(&[crc.checksum(&request_buffer)]).await.map_err(|_|RpcClientError::Serial)?;
                            };
                            result
                        };

                        let response_size = size_of::<<[< $name Response >] as rkyv::Archive>::Archived>();
                        let mut response_buffer = [0u8; size_of::<<[< $name Response >] as rkyv::Archive>::Archived>() + 1];

                        let rx_fut = async {
                            let result: Result<[< $name Response >], RpcClientError> = try {
                                log_info!("trying to read response {}", response_buffer.len());
                                match run_with_timeout(&mut self.delay, 1000.0, rx.read_exact(&mut response_buffer)).await{
                                    Ok(Ok(_))=>{}
                                    Ok(Err(_))=>{
                                        return Err(RpcClientError::Serial);
                                    }
                                    Err(_)=>{
                                        return Err(RpcClientError::Timeout);
                                    }
                                }
                                
                                let calculated_crc = crc.checksum(&response_buffer[..response_size]);
                                let received_crc = response_buffer[response_size];
                                if calculated_crc != received_crc {
                                    return Err(RpcClientError::ECCMismatch);
                                }
                                log_debug!("Response CRC matched.");
        
                                let archived = unsafe { archived_root::<[< $name Response >]>(&response_buffer[..response_size]) };
                                let deserialized = <[<Archived $name Response>] as rkyv::Deserialize<[< $name Response >], rkyv::Infallible>>::deserialize(archived, &mut rkyv::Infallible).unwrap();
                                deserialized
                            };
                            result
                        };
                        
                        let (tx_result, rx_result) = join!(tx_fut, rx_fut);
                        if tx_result.is_err() {
                            return Err(tx_result.unwrap_err());
                        }
                        
                        rx_result
                    }
                )*

            }
        }
    };
}
