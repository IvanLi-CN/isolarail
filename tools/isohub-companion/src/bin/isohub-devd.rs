use clap::{Parser, Subcommand};
use isohub_companion::{
    DEFAULT_BIND, DEFAULT_IPC_IDLE_TIMEOUT_SECS, DEFAULT_WEB_MDNS_NAME, DevdConfig, IpcConfig,
    default_ipc_endpoint, serve_http_bridge, serve_ipc,
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
    Web {
        #[arg(long, default_value = DEFAULT_BIND)]
        bind: SocketAddr,
        #[arg(long)]
        web_root: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_WEB_MDNS_NAME)]
        mdns_name: String,
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
        Command::Web {
            bind,
            web_root,
            mdns_name,
            allow_dev_cors,
        } => serve_http_bridge(DevdConfig::new(bind, web_root, allow_dev_cors, mdns_name)).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    #[test]
    fn web_command_accepts_bind_web_root_mdns_and_cors_flags() {
        let cli = Cli::try_parse_from([
            "isohub-devd",
            "web",
            "--bind",
            "127.0.0.1:51201",
            "--web-root",
            "web/dist",
            "--mdns-name",
            "isohub-devd",
            "--allow-dev-cors",
        ])
        .expect("web command should parse");

        let Command::Web {
            bind,
            web_root,
            mdns_name,
            allow_dev_cors,
        } = cli.command
        else {
            panic!("expected web command");
        };

        assert_eq!(bind.to_string(), "127.0.0.1:51201");
        assert_eq!(web_root, Some(PathBuf::from("web/dist")));
        assert_eq!(mdns_name, "isohub-devd");
        assert!(allow_dev_cors);
    }

    #[test]
    fn web_help_lists_expected_flags() {
        let mut command = Cli::command()
            .find_subcommand_mut("web")
            .expect("web subcommand should exist")
            .clone();
        let mut output = Vec::new();
        command.write_long_help(&mut output).expect("help writes");
        let help = String::from_utf8(output).expect("help should be utf8");

        assert!(help.contains("--bind"));
        assert!(help.contains("--web-root"));
        assert!(help.contains("--mdns-name"));
        assert!(help.contains("--allow-dev-cors"));
    }

    #[test]
    fn legacy_bridge_http_command_is_rejected() {
        let err = Cli::try_parse_from(["isohub-devd", "bridge-http"])
            .expect_err("legacy command should not parse");

        assert!(err.to_string().contains("unrecognized subcommand"));
    }
}
