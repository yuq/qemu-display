use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
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
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(flatten)]
    pub server: ServerArgs,

    /// DBUS address
    #[clap(short, long)]
    pub dbus_address: Option<String>,
}

pub fn parse() -> Args {
    Args::parse()
}
