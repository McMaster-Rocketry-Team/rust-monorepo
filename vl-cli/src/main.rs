use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use embedded_hal_async::delay::DelayNs;
use firmware_common::{driver::serial::SplitableSerialWrapper, RpcClient};
use tokio::io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::time::sleep;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use log::LevelFilter;

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

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::builder().filter_level(LevelFilter::Trace).try_init();

    let args = Cli::parse();

    if matches!(args.command, Commands::Detect) {
        println!("detect");
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
            println!("{:?}", args.file_type);
        }
        Commands::Pull(args) => {
            println!("{:?} {:?}", args.file_id, args.host_path);
        }
        Commands::Detect => unreachable!(),
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
