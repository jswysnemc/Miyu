use clap::Args;
use std::net::SocketAddr;

#[derive(Debug, Args)]
pub(crate) struct WebArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub(crate) listen: SocketAddr,
}
