use anyhow::{Context as _, anyhow};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use isohub_companion::{
    DeviceIdentity, DeviceProfile, DeviceRecord, FirmwareCatalog, HardwareTransport,
    SavedHardwareInput, api_url, default_ipc_endpoint, ipc_call, read_hardware_registry,
    redact_sensitive, registry_path, save_hardware,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::IsTerminal as _,
    path::PathBuf,
    process::{Command as ProcessCommand, Stdio},
    time::{Duration, Instant},
};

include!("isohub/cli.rs");
include!("isohub/app.rs");
include!("isohub/format.rs");
include!("isohub/platform.rs");
include!("isohub/discover.rs");
include!("isohub/tests.rs");
