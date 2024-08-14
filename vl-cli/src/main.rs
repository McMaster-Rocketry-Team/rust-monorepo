#![feature(generic_const_exprs)]

use std::path::PathBuf;
use std::time::Duration;

use crate::list_files::list_files;
use crate::pull_file::pull_file;
use crate::pull_vacuum_test::pull_vacuum_test;
use anyhow::anyhow;
use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use device_config::format_lora_key;
use device_config::gen_lora_key;
use device_config::read_device_config;
use embedded_hal_async::delay::DelayNs;
use firmware_common::common::console::vl_rpc::GCMPollDownlinkPacketResponse;
use firmware_common::common::vlp::packet::DeleteLogsPacket;
use firmware_common::common::vlp::packet::LowPowerModePacket;
use firmware_common::common::vlp::packet::ResetPacket;
use firmware_common::common::vlp::packet::SoftArmPacket;
use firmware_common::common::vlp::packet::VLPUplinkPacket;
use firmware_common::common::vlp::packet::VerticalCalibrationPacket;
use firmware_common::{driver::serial::SplitableSerialWrapper, vl_rpc::RpcClient};
use flight_profile::read_flight_profile;
use log::LevelFilter;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::time::sleep;
use tokio_serial::available_ports;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use vlfs::FileID;
use vlfs::FileType;

mod device_config;
mod flight_profile;
mod list_files;
mod pull_delta_logs;
mod pull_file;
mod pull_logs;
mod pull_vacuum_test;
mod reader;

struct Delay;

impl DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        sleep(Duration::from_nanos(ns as u64)).await;
    }
}

#[derive(Parser)]
#[command(name = "VL CLI")]
#[command(bin_name = "vl-cli")]
struct Cli {
    #[clap(subcommand)]
    mode: ModeSelect,
}

#[derive(Subcommand)]
enum ModeSelect {
    #[command(about = "List all the devices connected to the host")]
    Detect,

    #[command(about = "Void Lake specific commands")]
    VL(VLCli),

    #[command(about = "OZYS specific commands")]
    OZYS(SGCli),

    #[command(about = "Generate a new Lora key")]
    GenLoraKey,
}

#[derive(Parser)]
struct VLCli {
    serial: PathBuf,

    #[clap(subcommand)]
    command: VLCommands,
}

#[derive(Parser)]
struct SGCli {
    serial: PathBuf,

    #[clap(subcommand)]
    command: SGCommands,
}

#[derive(Subcommand)]
enum SGCommands {
    #[command(about = "Pull all the strain gauges readings from device")]
    PullData(PullDataArgs),

    LS(LSArgs),
    PullFile(PullArgs),

    #[command(about = "Reset device")]
    Reset,
}

#[derive(Subcommand)]
enum VLCommands {
    GCMSendUplink(SendUplinkArgs),
    GCMListen(GCMArgs),
    SetFlightProfile(FlightProfileArgs),
    SetDeviceConfig(DeviceConfigArgs),

    #[command(about = "Pull flight data from device")]
    PullFlight(PullDataArgs),

    #[command(about = "Pull vacuum test data from device")]
    PullVacuumTest(PullDataArgs),

    #[command(about = "Pull ground test data from device")]
    PullGroundTest(PullDataArgs),

    LS(LSArgs),
    PullFile(PullArgs),

    #[command(about = "Reset device")]
    Reset,
}

fn file_type_parser(s: &str) -> Result<FileType, String> {
    maybe_hex(s).map(FileType)
}

#[derive(clap::Args)]
#[command(about = "List files on the device")]
struct LSArgs {
    #[arg(value_parser=file_type_parser)]
    file_type: Option<FileType>,
}

fn file_id_parser(s: &str) -> Result<FileID, String> {
    maybe_hex(s).map(FileID)
}

#[derive(clap::Args)]
#[command(about = "Pull a file from the device")]
struct PullArgs {
    #[arg(value_parser=file_id_parser)]
    file_id: FileID,
    host_path: std::path::PathBuf,
}

#[derive(clap::Args)]
#[command(about = "Send VLP Uplink packet")]
struct SendUplinkArgs {
    command: String,
}

#[derive(clap::Args)]
#[command(about = "Listen on VLP Downlink packet")]
struct GCMArgs {}

#[derive(clap::Args)]
#[command(about = "Set flight profile")]
struct FlightProfileArgs {
    profile_path: std::path::PathBuf,
}

#[derive(clap::Args)]
#[command(about = "Set device config")]
struct DeviceConfigArgs {
    config_path: std::path::PathBuf,
}

#[derive(clap::Args)]
struct PullDataArgs {
    save_path: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .try_init();

    let args = Cli::parse();

    // if matches!(args.command, VLCommands::Detect) {
    //     for port in available_ports().unwrap() {
    //         println!("{:?}", port);
    //     }
    //     return Ok(());
    // }

    // if args.serial.is_none() {
    //     eprintln!("No serial port specified");
    //     return Ok(());
    // }
    // let serial: tokio_serial::SerialStream =
    //     tokio_serial::new(args.serial.unwrap(), 9600).open_native_async()?;
    // let (rx, tx) = split(serial);
    // let mut serial = SplitableSerialWrapper::new(SerialTXWrapper(tx), SerialRXWrapper(rx));
    // let mut client = RpcClient::new(&mut serial, Delay);
    // client.reset().await.map_err(|_| anyhow!("reset Error"))?;

    // let who_am_i = client.who_am_i().await.unwrap();
    // println!("Connected to {:?}", who_am_i.serial_number);

    // match args.command {
    //     VLCommands::LS(args) => {
    //         client.start_list_files(args.file_type).await.unwrap();
    //         println!("Files:");
    //         let file_ids = list_files(&mut client, args).await.unwrap();
    //         for file_id in file_ids {
    //             println!("File: {:?}", file_id);
    //         }
    //     }
    //     VLCommands::Pull(args) => {
    //         pull_file(&mut client, args).await.unwrap();
    //     }
    //     VLCommands::Detect => todo!(),
    //     VLCommands::SendUplink(SendUplinkArgs { command }) => {
    //         let packet: VLPUplinkPacket = match command.as_str() {
    //             "vertical_calibration" => VerticalCalibrationPacket { timestamp: 0.0 }.into(),
    //             "soft_arm" => SoftArmPacket {
    //                 timestamp: 0.0,
    //                 armed: true,
    //             }
    //             .into(),

    //             "soft_disarm" => SoftArmPacket {
    //                 timestamp: 0.0,
    //                 armed: false,
    //             }
    //             .into(),
    //             "low_power_mode_on" => LowPowerModePacket {
    //                 timestamp: 0.0,
    //                 enabled: true,
    //             }
    //             .into(),
    //             "low_power_mode_off" => LowPowerModePacket {
    //                 timestamp: 0.0,
    //                 enabled: false,
    //             }
    //             .into(),
    //             "reset" => ResetPacket { timestamp: 0.0 }.into(),
    //             "delete_logs" => DeleteLogsPacket { timestamp: 0.0 }.into(),
    //             _ => {
    //                 return Err(anyhow!("Invalid command"));
    //             }
    //         };
    //         let result = client.g_c_m_send_uplink_packet(packet).await.unwrap();
    //         println!("{:?}", result);
    //         loop {
    //             match client.g_c_m_poll_downlink_packet().await {
    //                 Ok(GCMPollDownlinkPacketResponse {
    //                     packet: Some((packet, status)),
    //                 }) => {
    //                     println!("{:?} {:?}", packet, status);
    //                 }
    //                 Err(e) => {
    //                     println!("{:?}", e);
    //                 }
    //                 _ => {}
    //             }
    //             sleep(Duration::from_millis(100)).await;
    //         }
    //     }
    //     VLCommands::GCM(_) => loop {
    //         match client.g_c_m_poll_downlink_packet().await {
    //             Ok(GCMPollDownlinkPacketResponse {
    //                 packet: Some((packet, status)),
    //             }) => {
    //                 println!("{:?} {:?}", packet, status);
    //             }
    //             Err(e) => {
    //                 println!("{:?}", e);
    //             }
    //             _ => {}
    //         }
    //         sleep(Duration::from_millis(100)).await;
    //     },
    //     VLCommands::FlightProfile(args) => {
    //         let profile = read_flight_profile(args.profile_path).unwrap();
    //         client.set_flight_profile(profile).await.unwrap();
    //     }
    //     VLCommands::DeviceConfig(args) => {
    //         let config = read_device_config(args.config_path).unwrap();
    //         client.set_device_config(config).await.unwrap();
    //     }
    //     VLCommands::GenLoraKey => {
    //         let key = gen_lora_key();
    //         println!("{}", format_lora_key(&key));
    //     }
    //     VLCommands::PullVacuumTest(args) => {
    //         pull_vacuum_test(&mut client, args).await.unwrap();
    //     }
    // }
    Ok(())
}

#[derive(defmt::Format, Debug)]
struct SerialErrorWrapper(#[defmt(Debug2Format)] std::io::Error);

impl embedded_io_async::Error for SerialErrorWrapper {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Other
    }
}

struct SerialRXWrapper(ReadHalf<SerialStream>);

impl embedded_io_async::ErrorType for SerialRXWrapper {
    type Error = SerialErrorWrapper;
}

impl embedded_io_async::Read for SerialRXWrapper {
    async fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(SerialErrorWrapper)
    }
}

struct SerialTXWrapper(WriteHalf<SerialStream>);

impl embedded_io_async::ErrorType for SerialTXWrapper {
    type Error = SerialErrorWrapper;
}

impl embedded_io_async::Write for SerialTXWrapper {
    async fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, Self::Error> {
        self.0.write(buf).await.map_err(SerialErrorWrapper)
    }
}
