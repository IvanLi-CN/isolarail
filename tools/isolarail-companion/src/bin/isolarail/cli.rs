#[derive(Debug, Parser)]
#[command(
    name = "isolarail",
    version = isolarail_companion::release_version(),
    about = "IsolaRail CLI"
)]
struct Cli {
    #[arg(long, global = true, default_value_t = default_ipc_endpoint())]
    ipc: String,
    #[arg(long, global = true)]
    no_auto_start: bool,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Discover {
        #[arg(long)]
        scan: bool,
    },
    Devices,
    Status(ApiSelectorArgs),
    DiagSnapshot(ApiSelectorArgs),
    Hardware {
        #[command(subcommand)]
        command: HardwareCommand,
    },
    Wifi {
        #[command(subcommand)]
        command: WifiCommand,
    },
    Ports {
        #[command(flatten)]
        selector: ApiSelectorArgs,
        #[command(subcommand)]
        command: Option<PortsCommand>,
    },
    Flash(FlashArgs),
    Reset(UsbSelectorArgs),
    Monitor {
        #[command(flatten)]
        selector: UsbSelectorArgs,
        #[arg(long, default_value_t = 200)]
        tail: usize,
    },
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
}

#[derive(Debug, clap::Args, Clone)]
struct ApiSelectorArgs {
    #[arg(long)]
    hardware: Option<String>,
    #[arg(long)]
    device: Option<String>,
    #[arg(long)]
    url: Option<String>,
}

impl ApiSelectorArgs {
    fn selection_count(&self) -> u8 {
        self.hardware.is_some() as u8 + self.device.is_some() as u8 + self.url.is_some() as u8
    }
}

#[derive(Debug, clap::Args, Clone)]
struct UsbSelectorArgs {
    #[arg(long)]
    hardware: Option<String>,
    #[arg(long)]
    device: Option<String>,
}

#[derive(Debug, Subcommand)]
enum HardwareCommand {
    Available {
        #[arg(long)]
        scan: bool,
    },
    Recent,
    List,
    Path,
    Save {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, value_enum)]
        transport: TransportArg,
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        url: Option<String>,
    },
    Forget {
        id: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum TransportArg {
    Usb,
    Http,
    WebSerial,
}

#[derive(Debug, Subcommand)]
enum WifiCommand {
    Show(ApiSelectorArgs),
    Set {
        #[command(flatten)]
        selector: ApiSelectorArgs,
        #[arg(long)]
        ssid: String,
        #[arg(long)]
        psk: String,
    },
    Clear(ApiSelectorArgs),
}

#[derive(Debug, Subcommand)]
enum PortsCommand {
    Power {
        #[arg(long, value_enum)]
        port: PortArg,
        #[arg(long, value_parser = clap::value_parser!(bool), action = ArgAction::Set)]
        enabled: bool,
    },
    Replug {
        #[arg(long, value_enum)]
        port: PortArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PortArg {
    Port1,
    Port2,
    Port3,
    Port4,
}

impl PortArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Port1 => "port1",
            Self::Port2 => "port2",
            Self::Port3 => "port3",
            Self::Port4 => "port4",
        }
    }
}

#[derive(Debug, clap::Args)]
struct FlashArgs {
    #[command(flatten)]
    selector: UsbSelectorArgs,
    #[arg(long)]
    catalog: PathBuf,
    #[arg(long)]
    artifact: String,
    #[arg(long)]
    real: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    first_time: bool,
    #[arg(long)]
    confirm_non_project_firmware: bool,
    #[arg(long)]
    expected_device_id: Option<String>,
    #[arg(long)]
    expected_mac: Option<String>,
}

#[derive(Debug, Subcommand)]
enum DiagnosticsCommand {
    Export(ApiSelectorArgs),
}

#[derive(Debug)]
struct UserCancelled;

impl std::fmt::Display for UserCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for UserCancelled {}

#[derive(Debug, Clone)]
struct DevdClient {
    endpoint: String,
    auto_start: bool,
}

impl DevdClient {
    fn with_endpoint(&self, endpoint: String) -> Self {
        Self {
            endpoint,
            auto_start: self.auto_start,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CliLease {
    lease_id: String,
    heartbeat_interval_ms: u64,
}
