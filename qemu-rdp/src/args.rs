use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// QEMU "-display dbus" RDP server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Print program capabilities in JSON.
    #[arg(long)]
    pub print_capabilities: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// D-Bus bus address (default to session)
    #[arg(short, long)]
    pub dbus_address: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start a RDP server
    #[command(arg_required_else_help = true)]
    Serve(ServerArgs),
}

#[derive(Debug, clap::Args)]
#[command(flatten_help = true)]
pub struct ServerArgs {
    /// IP address
    #[clap(short, long, default_value = "0.0.0.0:3389")]
    pub bind_addr: std::net::SocketAddr,

    /// Path to tls certificate
    #[clap(short, long, value_parser)]
    pub cert: PathBuf,

    /// Path to private key
    #[clap(short, long, value_parser)]
    pub key: PathBuf,

    /// RemoteFx encoding
    #[clap(value_enum, long, default_value = "enable")]
    pub remotefx: EnableDisableArg,
}

#[derive(Debug, PartialEq, Clone, ValueEnum)]
pub enum EnableDisableArg {
    Enable,
    Disable,
}

impl EnableDisableArg {
    pub fn enabled(&self) -> bool {
        matches!(self, Self::Enable)
    }
}
