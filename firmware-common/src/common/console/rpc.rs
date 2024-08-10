use crate::avionics::flight_profile::FlightProfile;
use crate::common::vl_device_manager::prelude::*;
use crate::common::file_types::{DEVICE_CONFIG_FILE_TYPE, FLIGHT_PROFILE_FILE_TYPE};
use crate::common::rkyv_structs::RkyvString;
use crate::common::rpc_channel::RpcChannelClient;
use crate::common::vlp::packet::VLPDownlinkPacket;
use crate::common::vlp::packet::VLPUplinkPacket;
use crate::create_rpc;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use lora_phy::mod_params::PacketStatus;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::ConcurrentFilesIterator;
use vlfs::{AsyncReader, Crc, FileID, FileReader, FileType, Flash, VLFSError, VLFSReadStatus};
use crate::common::device_config::DeviceConfig;
use crate::common::config_file::ConfigFile;

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct RpcPacketStatus {
    pub rssi: i16,
    pub snr: i16,
}

impl From<PacketStatus> for RpcPacketStatus {
    fn from(status: PacketStatus) -> Self {
        RpcPacketStatus {
            rssi: status.rssi,
            snr: status.snr,
        }
    }
}

create_rpc! {
    enums {
        enum OpenFileStatus {
            Sucess,
            DoesNotExist,
            Error,
        }
    }
    state<F: Flash, C: Crc, D: SysReset>(
        services: &VLSystemServices<'_, '_, '_, '_, impl Delay, impl Clock, F, C, D>,
        config: &Option<DeviceConfig>,
        device_serial_number: &[u8; 12],
        downlink_package_receiver: Receiver<'_, NoopRawMutex, (VLPDownlinkPacket, PacketStatus), 1>,
        send_uplink_packet_rpc_client: RpcChannelClient<
            '_,
            NoopRawMutex,
            VLPUplinkPacket,
            Option<PacketStatus>,
        >
    ) {
        let mut send_uplink_packet_rpc_client = send_uplink_packet_rpc_client;
        let fs = &services.fs;

        let mut reader: Option<FileReader<F, C>> = None;
        let mut file_iter: Option<ConcurrentFilesIterator<F, C, Option<FileType>>> = None;
    }
    rpc 0 WhoAmI | | -> (name: Option<RkyvString<64>>, serial_number: [u8; 12]) {
        WhoAmIResponse {
            name: config.as_ref().map(|config| config.name.clone()),
            serial_number: device_serial_number.clone(),
        }
    }
    rpc 1 OpenFile |file_id: u64| -> (status: OpenFileStatus) {
        let status = match fs.open_file_for_read(FileID(file_id)).await {
            Ok(r) => {
                let old_reader = reader.replace(r);
                if let Some(old_reader) = old_reader {
                    old_reader.close().await;
                }
                OpenFileStatus::Sucess
            }
            Err(VLFSError::FileDoesNotExist) => OpenFileStatus::DoesNotExist,
            Err(e) => {
                log_warn!("Error opening file: {:?}", e);
                OpenFileStatus::Error
            }
        };
        OpenFileResponse { status }
    }
    rpc 2 ReadFile | | -> (data: [u8; 128], length: u8, corrupted: bool) {
        let response = if let Some(reader) = reader.as_mut() {
            let mut buffer = [0u8; 128];
            match reader.read_all(&mut buffer).await {
                Ok((read_buffer, read_status)) => ReadFileResponse {
                    length: read_buffer.len() as u8,
                    data: buffer,
                    corrupted: matches!(read_status, VLFSReadStatus::CorruptedPage { .. }),
                },
                Err(e) => {
                    log_warn!("Error reading file: {:?}", e);
                    ReadFileResponse {
                        length: 0,
                        data: buffer,
                        corrupted: true,
                    }
                }
            }
        } else {
            ReadFileResponse {
                length: 0,
                data: [0u8; 128],
                corrupted: false,
            }
        };
        response
    }
    rpc 3 CloseFile | | -> () {
        if let Some(reader) = reader.take() {
            reader.close().await;
        }
    }
    rpc 4 StartListFiles |file_type: Option<u16>| -> () {
        file_iter = Some(fs.concurrent_files_iter(file_type.map(FileType)).await);
        StartListFilesResponse {}
    }
    rpc 5 GetListedFile | | -> (file_id: Option<u64>) {
        if let Some(file_iter) = &mut file_iter {
            match file_iter.next().await {
                Ok(Some(file)) => {
                    GetListedFileResponse {
                        file_id: Some(file.id.0),
                    }
                }
                Ok(None) => {
                    GetListedFileResponse { file_id: None }
                }
                Err(_) => {
                    GetListedFileResponse { file_id: None }
                }
            }
        }else{
            GetListedFileResponse { file_id: None }
        }
    }
    rpc 6 GCMSendUplinkPacket |packet: VLPUplinkPacket| -> (status: Option<RpcPacketStatus>) {
        let status = send_uplink_packet_rpc_client.call(packet).await;
        GCMSendUplinkPacketResponse {
            status: status.map(|status|status.into())
        }
    }
    rpc 7 GCMPollDownlinkPacket | | -> (packet: Option<(VLPDownlinkPacket, RpcPacketStatus)>) {
        GCMPollDownlinkPacketResponse {
            packet: downlink_package_receiver.try_receive().ok().map(|(packet, status)|(packet, status.into()))
        }
    }
    rpc 8 SetFlightProfile |
        flight_profile: FlightProfile
    | -> () {
        let flight_profile_file = ConfigFile::<FlightProfile, _, _>::new(services.fs, FLIGHT_PROFILE_FILE_TYPE);
        flight_profile_file.write(&flight_profile).await.unwrap();
        log_info!("Flight profile updated");
        SetFlightProfileResponse {}
    }
    rpc 9 SetDeviceConfig |
        device_config: DeviceConfig
    | -> () {
        let device_config_file = ConfigFile::<DeviceConfig, _, _>::new(services.fs, DEVICE_CONFIG_FILE_TYPE);
        device_config_file.write(&device_config).await.unwrap();
        log_info!("Device config updated");
        SetDeviceConfigResponse {}
    }
    rpc 10 ResetDevice | | -> () {
        services.reset();
        ResetDeviceResponse {}
    }
}
