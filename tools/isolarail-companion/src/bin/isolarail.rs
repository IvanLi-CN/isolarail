use anyhow::{Context as _, anyhow};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use isolarail_companion::{
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

include!("isolarail/cli.rs");
include!("isolarail/app.rs");
include!("isolarail/format.rs");
include!("isolarail/platform.rs");
include!("isolarail/discover.rs");
include!("isolarail/tests.rs");
