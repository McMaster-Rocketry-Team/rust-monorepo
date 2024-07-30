use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use crc::Crc;
use crc::CRC_8_SMBUS;
use embedded_hal_async::delay::DelayNs;
use firmware_common::common::console::rpc::GCMPollDownlinkPacketResponse;
use firmware_common::common::rkyv_structs::RkyvString;
use firmware_common::common::vlp2::packet2::DeleteLogsPacket;
use firmware_common::common::vlp2::packet2::LowPowerModePacket;
use firmware_common::common::vlp2::packet2::ResetPacket;
use firmware_common::common::vlp2::packet2::SoftArmPacket;
use firmware_common::common::vlp2::packet2::VLPUplinkPacket;
use firmware_common::common::vlp2::packet2::VerticalCalibrationPacket;
use firmware_common::{driver::serial::SplitableSerialWrapper, RpcClient};
use log::LevelFilter;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::time::sleep;
use tokio_serial::available_ports;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

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
    serial: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "List all the devices connected to the host")]
    Detect,
    LS(LSArgs),
    Pull(PullArgs),
    SendUplink(SendUplinkArgs),
    GCM(GCMArgs),
    FlightProfile(FlightProfileArgs),
    DeviceConfig(DeviceConfigArgs),
}

fn file_type_parser(s: &str) -> Result<u16, String> {
    maybe_hex(s)
}

#[derive(clap::Args)]
#[command(about = "List files")]
struct LSArgs {
    #[arg(value_parser=file_type_parser)]
    file_type: Option<u16>,
}

fn file_id_parser(s: &str) -> Result<u64, String> {
    maybe_hex(s)
}

#[derive(clap::Args)]
#[command(about = "Pull file from the device")]
struct PullArgs {
    #[arg(value_parser=file_id_parser)]
    file_id: u64,
    host_path: std::path::PathBuf,
}

#[derive(clap::Args)]
#[command(about = "Send VLP Uplink packet")]
struct SendUplinkArgs {
    command: String,
}

#[derive(clap::Args)]
#[command(about = "Pull VLP Downlink packet")]
struct GCMArgs {}

#[derive(clap::Args)]
#[command(about = "Set flight profile")]
struct FlightProfileArgs {
    drogue_pyro: u8,
    drogue_chute_minimum_time_ms: f64,
    drogue_chute_minimum_altitude_agl: f32,
    drogue_chute_delay_ms: f64,
    main_pyro: u8,
    main_chute_altitude_agl: f32,
    main_chute_delay_ms: f64,
}

#[derive(clap::Args)]
#[command(about = "Set device config")]
struct DeviceConfigArgs {
    #[clap(long, short, action)]
    avionics: bool,
    name: String,
    lora_frequency: u32,
    lora_sf: u8,
    lora_bw: u32,
    lora_cr: u8,
    lora_power: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .try_init();

    let crc = Crc::<u8>::new(&CRC_8_SMBUS);
    println!("{:?}", crc.checksum(&[]));

    let args = Cli::parse();

    if matches!(args.command, Commands::Detect) {
        for port in available_ports().unwrap(){
            println!("{:?}", port);
        }
        return Ok(());
    }

    if args.serial.is_none() {
        return Ok(());
    }
    let serial: tokio_serial::SerialStream =
        tokio_serial::new(args.serial.unwrap(), 9600).open_native_async()?;
    let (rx, tx) = split(serial);
    let mut serial = SplitableSerialWrapper::new(SerialTXWrapper(tx), SerialRXWrapper(rx));
    let mut client = RpcClient::new(&mut serial, Delay);
    client.reset().await.map_err(|_| anyhow!("reset Error"))?;

    let who_am_i = client.who_am_i().await.unwrap();
    println!("Connected to {:?}", who_am_i.serial_number);

    match args.command {
        Commands::LS(args) => {
            client.start_list_files(args.file_type).await.unwrap();
            println!("Files:");
            loop {
                let response = client.get_listed_file().await.unwrap();
                if let Some(file_id) = response.file_id {
                    println!("File: {:?}", file_id);
                } else {
                    break;
                }
            }
        }
        Commands::Pull(args) => {
            println!("{:?} {:?}", args.file_id, args.host_path);
        }
        Commands::Detect => todo!(),
        Commands::SendUplink(SendUplinkArgs { command }) => {
            let packet: VLPUplinkPacket = match command.as_str() {
                "vertical_calibration" => VerticalCalibrationPacket { timestamp: 0.0 }.into(),
                "soft_arm" => SoftArmPacket {
                    timestamp: 0.0,
                    armed: true,
                }
                .into(),

                "soft_disarm" => SoftArmPacket {
                    timestamp: 0.0,
                    armed: false,
                }
                .into(),
                "low_power_mode_on" => LowPowerModePacket {
                    timestamp: 0.0,
                    enabled: true,
                }
                .into(),
                "low_power_mode_off" => LowPowerModePacket {
                    timestamp: 0.0,
                    enabled: false,
                }
                .into(),
                "reset" => ResetPacket {
                    timestamp: 0.0,
                }.into(),
                "delete_logs" => DeleteLogsPacket {
                    timestamp: 0.0,
                }.into(),
                _ => {
                    return Err(anyhow!("Invalid command"));
                }
            };
            let result = client.g_c_m_send_uplink_packet(packet).await.unwrap();
            println!("{:?}", result);
            loop {
                match client.g_c_m_poll_downlink_packet().await {
                    Ok(GCMPollDownlinkPacketResponse {
                        packet: Some((packet, status)),
                    }) => {
                        println!("{:?} {:?}", packet, status);
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                    _ => {}
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
        Commands::GCM(_) => loop {
            match client.g_c_m_poll_downlink_packet().await {
                Ok(GCMPollDownlinkPacketResponse {
                    packet: Some((packet, status)),
                }) => {
                    println!("{:?} {:?}", packet, status);
                }
                Err(e) => {
                    println!("{:?}", e);
                }
                _ => {}
            }
            sleep(Duration::from_millis(100)).await;
        },
        Commands::FlightProfile(profile) => {
            client
                .set_flight_profile(
                    profile.drogue_pyro,
                    profile.drogue_chute_minimum_time_ms,
                    profile.drogue_chute_minimum_altitude_agl,
                    profile.drogue_chute_delay_ms,
                    profile.main_pyro,
                    profile.main_chute_altitude_agl,
                    profile.main_chute_delay_ms,
                )
                .await
                .unwrap();
        }
        Commands::DeviceConfig(config) => {
            let lora_key = [0x69u8; 32];
            client
                .set_device_config(
                    config.avionics,
                    RkyvString::from_str(&config.name),
                    lora_key,
                    config.lora_frequency,
                    config.lora_sf,
                    config.lora_bw,
                    config.lora_cr,
                    config.lora_power,
                )
                .await
                .unwrap();
        }
    }
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
