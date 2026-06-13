use clap::{Parser, Subcommand};
use isohub_companion::{
    DEFAULT_BIND, DEFAULT_IPC_IDLE_TIMEOUT_SECS, DevdConfig, IpcConfig, default_ipc_endpoint,
    serve_http_bridge, serve_ipc,
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Parser)]
#[command(
    name = "isohub-devd",
    version = isohub_companion::release_version(),
    about = "IsoHub local device daemon"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(long, default_value_t = default_ipc_endpoint())]
        endpoint: String,
        #[arg(long, default_value_t = DEFAULT_IPC_IDLE_TIMEOUT_SECS)]
        idle_timeout_secs: u64,
    },
    BridgeHttp {
        #[arg(long, default_value = DEFAULT_BIND)]
        bind: SocketAddr,
        #[arg(long)]
        web_root: Option<PathBuf>,
        #[arg(long)]
        allow_dev_cors: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Serve {
            endpoint,
            idle_timeout_secs,
        } => {
            let idle_timeout =
                (idle_timeout_secs > 0).then(|| Duration::from_secs(idle_timeout_secs));
            serve_ipc(IpcConfig::new(endpoint).with_idle_timeout(idle_timeout)).await?
        }
        Command::BridgeHttp {
            bind,
            web_root,
            allow_dev_cors,
        } => serve_http_bridge(DevdConfig::new(bind, web_root, allow_dev_cors)).await?,
    }
    Ok(())
}
