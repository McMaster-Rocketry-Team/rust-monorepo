use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;

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

fn main() -> Result<()> {
    // for port in tokio_serial::available_ports()? {
    //     println!("{:?}", port);
    // }
    let args = Cli::parse();
    println!("{:?}", args.serial);
    match args.command {
        Commands::Detect => {
            println!("detect");
        }
        Commands::LS(args) => {
            println!("{:?}", args.file_type);
        }
        Commands::Pull(args) => {
            println!("{:?} {:?}", args.file_id, args.host_path);
        }
    }
    Ok(())
}
