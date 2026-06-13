use anyhow::{Context as _, anyhow};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use directories::ProjectDirs;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use rand::{Rng as _, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{Mutex, OwnedMutexGuard},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
};

pub const DEFAULT_BIND: &str = "127.0.0.1:51200";
pub const DEFAULT_WEB_MDNS_NAME: &str = "isohub-devd";
pub const WEB_MDNS_SERVICE_TYPE: &str = "_isohub-devd._tcp.local.";
pub const DEFAULT_IPC_FILE_NAME: &str = "devd.sock";
pub const DEFAULT_WINDOWS_PIPE_NAME: &str = r"\\.\pipe\isohub-devd";
const STORAGE_FILE_NAME: &str = "devices.json";
const STORAGE_SETTINGS_FILE_NAME: &str = "settings.json";
const STORAGE_SCHEMA_VERSION: u8 = 1;
const DEFAULT_FLASH_ADDRESS: u64 = 0x10000;
const LEASE_TTL_MS: u64 = 8_000;
const LEASE_HEARTBEAT_INTERVAL_MS: u64 = 2_000;
const SERIAL_BAUD: u32 = 115_200;
const SERIAL_TIMEOUT_MS: u64 = 1_500;
const SERIAL_POWER_CONFIG_TIMEOUT_MS: u64 = 8_000;
const MAX_SESSION_ITEMS: usize = 500;
pub const DEFAULT_IPC_IDLE_TIMEOUT_SECS: u64 = 30;
const PROJECT_FIRMWARE_NAME: &str = "iso-usb-hub";
const MIN_COMPATIBLE_FIRMWARE_VERSION: &str = "0.1.0";

pub fn release_version() -> &'static str {
    option_env!("ISOHUB_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone)]
pub struct DevdConfig {
    pub bind: SocketAddr,
    pub web_root: Option<PathBuf>,
    pub allow_dev_cors: bool,
    pub mdns_name: String,
}

impl DevdConfig {
    pub fn new(
        bind: SocketAddr,
        web_root: Option<PathBuf>,
        allow_dev_cors: bool,
        mdns_name: impl Into<String>,
    ) -> Self {
        Self {
            bind,
            web_root,
            allow_dev_cors,
            mdns_name: mdns_name.into(),
        }
    }
}

pub struct WebMdnsAdvertiser {
    mdns: ServiceDaemon,
    fullname: String,
}

impl Drop for WebMdnsAdvertiser {
    fn drop(&mut self) {
        if let Err(err) = self.mdns.unregister(&self.fullname) {
            tracing::warn!(
                "isohub-devd web mDNS unregister failed (service={}): {}",
                self.fullname,
                err
            );
        }
    }
}

pub fn build_web_mdns_service_info(
    instance_name: &str,
    port: u16,
) -> anyhow::Result<ServiceInfo> {
    let properties = [
        ("app", "isohub-devd"),
        ("mode", "web"),
        ("version", release_version()),
        ("api", "/api/v1/bootstrap"),
    ];
    let host_name = format!("{instance_name}.local.");
    Ok(ServiceInfo::new(
        WEB_MDNS_SERVICE_TYPE,
        instance_name,
        host_name.as_str(),
        "",
        port,
        &properties[..],
    )?
    .enable_addr_auto())
}

pub fn advertise_web_mdns(instance_name: &str, port: u16) -> anyhow::Result<WebMdnsAdvertiser> {
    let mdns = ServiceDaemon::new().context("create mDNS daemon")?;
    let service_info = build_web_mdns_service_info(instance_name, port)?;
    let fullname = service_info.get_fullname().to_string();
    mdns.register(service_info)
        .with_context(|| format!("register mDNS service {fullname}"))?;
    Ok(WebMdnsAdvertiser { mdns, fullname })
}

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub endpoint: String,
    pub idle_timeout: Option<Duration>,
}

impl IpcConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            idle_timeout: Some(Duration::from_secs(DEFAULT_IPC_IDLE_TIMEOUT_SECS)),
        }
    }

    pub fn with_idle_timeout(mut self, idle_timeout: Option<Duration>) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn default_ipc_endpoint() -> String {
    #[cfg(windows)]
    {
        DEFAULT_WINDOWS_PIPE_NAME.to_string()
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join(format!("isohub-{}", user_id_hint())));
        base.join("isohub")
            .join(DEFAULT_IPC_FILE_NAME)
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(not(windows))]
fn user_id_hint() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

#[derive(Clone)]
struct AppState {
    token: String,
    base_url: String,
    inner: Arc<Mutex<DevdState>>,
}

impl AppState {
    fn new(base_url: impl Into<String>) -> Self {
        Self {
            token: generate_token(),
            base_url: base_url.into(),
            inner: Arc::new(Mutex::new(DevdState::default())),
        }
    }
}

#[derive(Clone)]
struct IpcRuntime {
    app: AppState,
    lifecycle: Arc<Mutex<IpcLifecycle>>,
}

impl IpcRuntime {
    fn new(app: AppState) -> Self {
        Self {
            app,
            lifecycle: Arc::new(Mutex::new(IpcLifecycle::default())),
        }
    }
}

struct IpcLifecycle {
    active_clients: usize,
    last_activity: Instant,
}

impl Default for IpcLifecycle {
    fn default() -> Self {
        Self {
            active_clients: 0,
            last_activity: Instant::now(),
        }
    }
}

#[derive(Default)]
struct DevdState {
    devices: HashMap<String, DeviceRecord>,
    leases: HashMap<String, LeaseRecord>,
    exclusive_ports: HashMap<String, String>,
    port_operation_locks: HashMap<String, Arc<Mutex<()>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub id: String,
    pub display_name: String,
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb: Option<UsbTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Value>,
    #[serde(default)]
    pub session: DeviceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbTarget {
    pub port_path: String,
    pub label: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpTarget {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceSession {
    #[serde(default)]
    pub logs: VecDeque<SessionItem>,
    #[serde(default)]
    pub traces: VecDeque<SessionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionItem {
    pub id: String,
    pub timestamp_unix_ms: u128,
    pub level: String,
    pub message: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialCdcTraceKind {
    Json,
    Raw,
    Defmt,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialCdcTrace {
    pub kind: SerialCdcTraceKind,
    pub summary: String,
    pub payload: String,
}

pub fn extract_json_frames_from_cdc_line(line: &[u8]) -> Vec<Value> {
    let mut frames = Vec::new();
    let mut start = 0usize;
    while start < line.len() {
        while start < line.len() && line[start] != b'{' {
            start += 1;
        }
        if start >= line.len() {
            break;
        }
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escape = false;
        let mut matched_end = None;
        for (offset, current) in line[start..].iter().enumerate() {
            if in_string {
                if escape {
                    escape = false;
                    continue;
                }
                match current {
                    b'\\' => escape = true,
                    b'"' => in_string = false,
                    _ => {}
                }
                continue;
            }
            match current {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = start + offset + 1;
                        let Ok(text) = std::str::from_utf8(&line[start..end]) else {
                            break;
                        };
                        if let Ok(value) = serde_json::from_str::<Value>(text) {
                            frames.push(value);
                        }
                        matched_end = Some(end);
                        break;
                    }
                }
                _ => {}
            }
        }
        start = matched_end.unwrap_or(start + 1);
    }
    frames
}

pub fn summarize_cdc_line(line: &[u8]) -> Vec<SerialCdcTrace> {
    let mut traces = Vec::new();
    let frames = extract_json_frames_from_cdc_line(line);
    if !frames.is_empty() {
        for frame in frames {
            let summary = if frame.get("type").and_then(Value::as_str) == Some("log") {
                let level = frame.get("level").and_then(Value::as_str).unwrap_or("info");
                let target = frame
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or("usb_jsonl");
                let message = frame
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("USB CDC log");
                format!("json log {level} {target}: {message}")
            } else if let Some(method) = frame.get("method").and_then(Value::as_str) {
                format!("json request {method}")
            } else if let Some(id) = frame.get("id").and_then(Value::as_str) {
                format!("json response {id}")
            } else {
                "json frame".to_string()
            };
            let payload =
                serde_json::to_string(&frame).unwrap_or_else(|_| "<invalid json>".to_string());
            traces.push(SerialCdcTrace {
                kind: SerialCdcTraceKind::Json,
                summary,
                payload,
            });
        }
        return traces;
    }

    let trimmed = trim_line_breaks(line);
    if trimmed.is_empty() {
        return traces;
    }

    if let Ok(text) = std::str::from_utf8(trimmed) {
        let text = text.trim();
        if !text.is_empty() {
            traces.push(SerialCdcTrace {
                kind: SerialCdcTraceKind::Raw,
                summary: "raw cdc line".to_string(),
                payload: text.to_string(),
            });
        }
        return traces;
    }

    traces.push(SerialCdcTrace {
        kind: SerialCdcTraceKind::Defmt,
        summary: "defmt/raw binary frame".to_string(),
        payload: hex_preview(trimmed, 96),
    });
    traces
}

fn trim_line_breaks(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && matches!(line[end - 1], b'\n' | b'\r') {
        end -= 1;
    }
    &line[..end]
}

pub fn hex_preview(bytes: &[u8], max_len: usize) -> String {
    let display = bytes.len().min(max_len);
    let mut preview = bytes[..display]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > max_len {
        preview.push_str(" …");
    }
    preview
}

#[derive(Debug, Clone)]
struct LeaseRecord {
    lease_id: String,
    device_id: String,
    port_path: Option<String>,
    expires_at: Instant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapResponse {
    token: String,
    agent_base_url: String,
    app: BootstrapApp,
}

#[derive(Debug, Serialize)]
struct BootstrapApp {
    name: &'static str,
    version: &'static str,
    mode: &'static str,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HardwareRegistry {
    pub schema_version: u8,
    #[serde(default)]
    pub devices: Vec<DeviceProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StorageSettings {
    theme: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub id: String,
    pub name: String,
    pub transport: HardwareTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<DeviceIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum HardwareTransport {
    Usb {
        device_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        devd_url: Option<String>,
    },
    Http {
        base_url: String,
    },
    WebSerial {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SavedHardwareInput {
    pub id: String,
    pub name: String,
    pub transport: HardwareTransport,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FirmwareCatalog {
    #[serde(rename = "schemaVersion", alias = "schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub artifacts: Vec<FirmwareArtifact>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareArtifact {
    pub artifact_id: String,
    pub target: String,
    pub version: String,
    #[serde(default)]
    pub git_sha: Option<String>,
    #[serde(default)]
    pub build_id: Option<String>,
    #[serde(default)]
    pub files: Vec<FirmwareFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFile {
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub flash_address: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LeaseRequest {
    device_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LeaseResponse {
    lease_id: String,
    device_id: String,
    heartbeat_interval_ms: u64,
    lease_ttl_ms: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceQuery {
    lease_id: Option<String>,
    tail: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WifiRequest {
    ssid: String,
    psk: String,
}

#[derive(Debug, Deserialize)]
struct FlashRequest {
    catalog_path: PathBuf,
    artifact_id: String,
    #[serde(default)]
    real: bool,
    #[serde(default)]
    first_time: bool,
    #[serde(default)]
    confirm_non_project_firmware: bool,
    #[serde(default)]
    expected_identity: Option<DeviceIdentity>,
    lease_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareUploadFlashRequest {
    address: u64,
    file_name: String,
    file_base64: String,
    expected_identity: DeviceIdentity,
    lease_id: String,
}

#[derive(Debug, Deserialize)]
struct StorageImportRequest {
    #[serde(default)]
    devices: Vec<Value>,
    #[serde(default)]
    profiles: Vec<DeviceProfile>,
    #[serde(default)]
    settings: Option<StorageSettings>,
}

#[derive(Debug, Deserialize)]
struct StorageSettingsRequest {
    settings: StorageSettings,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorInfo,
}

#[derive(Debug, Serialize)]
struct ErrorInfo {
    code: &'static str,
    message: String,
    retryable: bool,
}

include!("lib/ipc.rs");
include!("lib/http_bridge.rs");

include!("lib/device_io.rs");

include!("lib/storage_catalog.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_nested_sensitive_fields() {
        let value = json!({
            "ssid": "bench",
            "psk": "secret",
            "nested": {"token": "abc", "ok": true},
        });
        let redacted = redact_sensitive(&value);
        assert_eq!(redacted["psk"], "<redacted>");
        assert_eq!(redacted["nested"]["token"], "<redacted>");
        assert_eq!(redacted["nested"]["ok"], true);
    }

    #[test]
    fn prunes_stale_usb_devices_after_scan() {
        let mut inner = DevdState::default();
        reconcile_scanned_usb_devices(
            &mut inner,
            vec![UsbTarget {
                port_path: "/dev/cu.usbmodem101".to_string(),
                label: "ESP32-S3 USB JTAG".to_string(),
                vendor_id: Some(0x303a),
                product_id: Some(0x1001),
                serial_number: None,
            }],
        );
        assert!(inner.devices.contains_key("usb--dev-cu-usbmodem101"));

        reconcile_scanned_usb_devices(&mut inner, Vec::new());
        assert!(!inner.devices.contains_key("usb--dev-cu-usbmodem101"));
    }

    #[test]
    fn scan_keeps_http_profile_when_usb_channel_disappears() {
        let mut inner = DevdState::default();
        inner.devices.insert(
            "combo".to_string(),
            DeviceRecord {
                id: "combo".to_string(),
                display_name: "Bench Hub".to_string(),
                connection: "available".to_string(),
                usb: Some(UsbTarget {
                    port_path: "/dev/cu.usbmodem101".to_string(),
                    label: "ESP32-S3 USB JTAG".to_string(),
                    vendor_id: Some(0x303a),
                    product_id: Some(0x1001),
                    serial_number: None,
                }),
                http: Some(HttpTarget {
                    base_url: "http://isohub.local".to_string(),
                }),
                identity: None,
                session: DeviceSession::default(),
            },
        );

        reconcile_scanned_usb_devices(&mut inner, Vec::new());
        let device = inner.devices.get("combo").expect("profile remains");
        assert!(device.usb.is_none());
        assert_eq!(device.connection, "unavailable");
    }

    #[test]
    fn dedupes_macos_tty_cu_pairs_and_prefers_cu() {
        let targets = dedupe_usb_serial_device_pairs(vec![
            UsbTarget {
                port_path: "/dev/tty.usbmodem101".to_string(),
                label: "ESP32-S3 USB JTAG".to_string(),
                vendor_id: Some(0x303a),
                product_id: Some(0x1001),
                serial_number: None,
            },
            UsbTarget {
                port_path: "/dev/cu.usbmodem101".to_string(),
                label: "ESP32-S3 USB JTAG".to_string(),
                vendor_id: Some(0x303a),
                product_id: Some(0x1001),
                serial_number: None,
            },
        ]);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].port_path, "/dev/cu.usbmodem101");
    }

    #[test]
    fn usb_port_allowlist_filters_non_matching_ports() {
        unsafe {
            std::env::set_var("ISOHUB_USB_PORT", "/dev/cu.usbmodem2123101");
        }
        let allowed = ensure_port_allowed("/dev/cu.usbmodem2123101");
        let blocked = ensure_port_allowed("/dev/cu.usbmodem9999999");
        unsafe {
            std::env::remove_var("ISOHUB_USB_PORT");
        }

        assert!(allowed.is_ok());
        let err = blocked.expect_err("other ports must be rejected");
        assert!(
            err.to_string()
                .contains("serial port /dev/cu.usbmodem9999999 is not allowed")
        );
    }

    #[test]
    fn upsert_profile_preserves_existing_identity_when_incoming_omits_it() {
        let mut registry = HardwareRegistry {
            schema_version: STORAGE_SCHEMA_VERSION,
            devices: vec![DeviceProfile {
                id: "bench".to_string(),
                name: "Bench".to_string(),
                transport: HardwareTransport::Usb {
                    device_id: "usb--dev-cu-usbmodem101".to_string(),
                    devd_url: None,
                },
                identity: Some(DeviceIdentity {
                    device_id: Some("isohub-abc".to_string()),
                    mac: Some("AA:BB:CC:DD:EE:FF".to_string()),
                }),
                last_seen_at: Some(1),
            }],
        };

        upsert_profile(
            &mut registry,
            DeviceProfile {
                id: "bench".to_string(),
                name: "Bench renamed".to_string(),
                transport: HardwareTransport::Usb {
                    device_id: "usb--dev-cu-usbmodem101".to_string(),
                    devd_url: None,
                },
                identity: None,
                last_seen_at: Some(2),
            },
        );

        assert_eq!(registry.devices[0].name, "Bench renamed");
        assert_eq!(
            registry.devices[0]
                .identity
                .as_ref()
                .and_then(|identity| identity.device_id.as_deref()),
            Some("isohub-abc")
        );
    }

    #[test]
    fn web_storage_coalesces_usb_and_default_wifi_hostname_profiles() {
        let registry = HardwareRegistry {
            schema_version: STORAGE_SCHEMA_VERSION,
            devices: vec![
                DeviceProfile {
                    id: "isohub-01".to_string(),
                    name: "isohub-01".to_string(),
                    transport: HardwareTransport::Usb {
                        device_id: "usb--dev-cu-usbmodem21221401".to_string(),
                        devd_url: None,
                    },
                    identity: Some(DeviceIdentity {
                        device_id: Some("856a14".to_string()),
                        mac: Some("1c:db:d4:85:6a:14".to_string()),
                    }),
                    last_seen_at: Some(10),
                },
                DeviceProfile {
                    id: "isohub-01-wifi".to_string(),
                    name: "isohub-01 Wi-Fi".to_string(),
                    transport: HardwareTransport::Http {
                        base_url: "http://isohub-856a14.local".to_string(),
                    },
                    identity: None,
                    last_seen_at: Some(11),
                },
            ],
        };

        let devices = web_storage_devices(&registry);

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0]["id"], "isohub-01");
        assert_eq!(devices[0]["name"], "isohub-01");
        assert_eq!(devices[0]["baseUrl"], "http://isohub-856a14.local");
        assert_eq!(
            devices[0]["transports"]["httpBaseUrl"],
            "http://isohub-856a14.local"
        );
        assert_eq!(
            devices[0]["transports"]["localUsbDeviceId"],
            "usb--dev-cu-usbmodem21221401"
        );
    }

    #[test]
    fn validates_catalog_shape() {
        let catalog = FirmwareCatalog {
            schema_version: "1".to_string(),
            artifacts: vec![FirmwareArtifact {
                artifact_id: "app".to_string(),
                target: "esp32s3_app".to_string(),
                version: "v1".to_string(),
                git_sha: None,
                build_id: None,
                files: vec![FirmwareFile {
                    kind: "app_bin".to_string(),
                    path: "app.bin".to_string(),
                    sha256: "a".repeat(64),
                    size: 1,
                    flash_address: Some(DEFAULT_FLASH_ADDRESS),
                }],
            }],
        };
        assert!(validate_catalog_shape(&catalog).is_empty());
    }

    #[test]
    fn rejects_wrong_app_address() {
        let catalog = FirmwareCatalog {
            schema_version: "1".to_string(),
            artifacts: vec![FirmwareArtifact {
                artifact_id: "app".to_string(),
                target: "esp32s3_app".to_string(),
                version: "v1".to_string(),
                git_sha: None,
                build_id: None,
                files: vec![FirmwareFile {
                    kind: "app_bin".to_string(),
                    path: "app.bin".to_string(),
                    sha256: "a".repeat(64),
                    size: 1,
                    flash_address: Some(0),
                }],
            }],
        };
        assert!(!validate_catalog_shape(&catalog).is_empty());
    }

    #[test]
    fn rejects_wrong_full_image_address() {
        let catalog = FirmwareCatalog {
            schema_version: "1".to_string(),
            artifacts: vec![FirmwareArtifact {
                artifact_id: "full".to_string(),
                target: "esp32s3_full".to_string(),
                version: "v1".to_string(),
                git_sha: None,
                build_id: None,
                files: vec![FirmwareFile {
                    kind: "full_image".to_string(),
                    path: "full.bin".to_string(),
                    sha256: "a".repeat(64),
                    size: 1,
                    flash_address: Some(DEFAULT_FLASH_ADDRESS),
                }],
            }],
        };
        assert!(!validate_catalog_shape(&catalog).is_empty());
    }

    #[test]
    fn validates_expected_device_identity() {
        let info = json!({
            "ok": true,
            "result": {
                "device": {
                    "device_id": "abc123",
                    "mac": "AA:BB:CC:DD:EE:FF"
                }
            }
        });
        validate_device_identity(
            &info,
            &DeviceIdentity {
                device_id: Some("abc123".to_string()),
                mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
            },
        )
        .expect("identity should match");
    }

    #[test]
    fn validates_project_firmware_name_and_version() {
        let info = json!({
            "ok": true,
            "result": {
                "device": {
                    "firmware": {
                        "name": "iso-usb-hub",
                        "version": "0.1.0"
                    }
                }
            }
        });
        validate_project_firmware(&info).expect("project firmware should pass");
    }

    #[test]
    fn rejects_non_project_or_incompatible_firmware() {
        let wrong_name = json!({
            "result": {
                "device": {
                    "firmware": {
                        "name": "other",
                        "version": "0.1.0"
                    }
                }
            }
        });
        assert!(validate_project_firmware(&wrong_name).is_err());

        let old_version = json!({
            "result": {
                "device": {
                    "firmware": {
                        "name": "iso-usb-hub",
                        "version": "0.0.1"
                    }
                }
            }
        });
        assert!(validate_project_firmware(&old_version).is_err());

        let firmware = project_firmware_metadata(&old_version).expect("firmware metadata");
        validate_project_firmware_name(firmware)
            .expect("upgrade path accepts old project firmware");
    }

    #[test]
    fn rejects_mismatched_device_identity() {
        let info = json!({"result": {"device": {"device_id": "abc123"}}});
        assert!(
            validate_device_identity(
                &info,
                &DeviceIdentity {
                    device_id: Some("other".to_string()),
                    mac: None,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn matches_wifi_set_verification_shape() {
        let value = json!({
            "ok": true,
            "result": {
                "configured": true,
                "ssid": "Ivan",
                "state": "connected"
            }
        });
        assert!(wifi_matches_expected_ssid(&value, "Ivan"));
        assert!(!wifi_matches_expected_ssid(&value, "Other"));
    }

    #[test]
    fn import_accepts_exported_profiles_shape() {
        let req = StorageImportRequest {
            devices: vec![json!({
                "id": "web",
                "name": "Web device",
                "baseUrl": "isohub-devd://usb--dev-cu-usbmodem101"
            })],
            profiles: vec![DeviceProfile {
                id: "cli".to_string(),
                name: "CLI device".to_string(),
                transport: HardwareTransport::Usb {
                    device_id: "usb--dev-cu-usbmodem101".to_string(),
                    devd_url: None,
                },
                identity: None,
                last_seen_at: Some(1),
            }],
            settings: Some(StorageSettings {
                theme: "isohub-dark".to_string(),
            }),
        };
        let profiles =
            parse_import_profiles(&req).expect("profiles should be preferred when exported");

        assert_eq!(
            req.settings
                .as_ref()
                .map(|settings| settings.theme.as_str()),
            Some("isohub-dark")
        );
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "cli");

        let devices = parse_import_profiles(&StorageImportRequest {
            devices: vec![json!({
                "id": "web",
                "name": "Web device",
                "baseUrl": "isohub-devd://usb--dev-cu-usbmodem101"
            })],
            profiles: vec![],
            settings: None,
        })
        .expect("web devices should import");

        assert!(matches!(
            devices[0].transport,
            HardwareTransport::Usb { ref device_id, .. }
                if device_id == "usb--dev-cu-usbmodem101"
        ));
    }

    #[test]
    fn serial_timeout_summary_prefers_recent_non_json_activity() {
        let summary = summarize_serial_timeout(&[
            json!({
                "kind": "defmt",
                "summary": "defmt/raw binary frame",
                "payload": "ff001f..."
            }),
            json!({
                "kind": "ignored",
                "summary": "serial response timed out with trailing bytes",
                "payload": "ff001f..."
            }),
        ])
        .expect("summary should exist");

        assert!(summary.contains("defmt: defmt/raw binary frame"));
        assert!(summary.contains("ignored: serial response timed out with trailing bytes"));
    }

    #[tokio::test]
    async fn local_usb_operation_lock_rejects_second_concurrent_request() {
        let lock = Arc::new(Mutex::new(()));
        let guard = acquire_port_operation_lock_with_timeout(
            "/dev/cu.usbmodem2123101",
            lock.clone(),
            Duration::from_millis(20),
        )
        .await
        .expect("first request should acquire the port lock");

        let err = acquire_port_operation_lock_with_timeout(
            "/dev/cu.usbmodem2123101",
            lock,
            Duration::from_millis(20),
        )
        .await
        .expect_err("second concurrent request should be rejected");

        assert!(
            err.to_string()
                .contains("device busy: another Local USB operation is still running"),
            "unexpected error: {err}"
        );

        drop(guard);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ipc_serves_jsonl_requests_over_unix_socket() {
        let temp = tempfile::tempdir().expect("temp dir");
        let endpoint = temp.path().join("devd.sock");
        let endpoint_string = endpoint.to_string_lossy().to_string();
        let task = tokio::spawn({
            let endpoint = endpoint_string.clone();
            async move { serve_ipc(IpcConfig::new(endpoint)).await }
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut last_error = None;
        let result = loop {
            match ipc_call(&endpoint_string, "devd.health", json!({})).await {
                Ok(value) => break value,
                Err(err) if Instant::now() < deadline => {
                    last_error = Some(err);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                Err(err) => panic!(
                    "IPC health failed: {err}; last={}",
                    last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "none".to_string())
                ),
            }
        };
        task.abort();
        assert_eq!(result["ok"], true);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ipc_daemon_exits_after_idle_timeout() {
        let temp = tempfile::tempdir().expect("temp dir");
        let endpoint = temp.path().join("devd.sock");
        let endpoint_string = endpoint.to_string_lossy().to_string();
        let task = tokio::spawn({
            let endpoint = endpoint_string.clone();
            async move {
                serve_ipc(
                    IpcConfig::new(endpoint).with_idle_timeout(Some(Duration::from_millis(100))),
                )
                .await
            }
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if endpoint.exists() {
                break;
            }
            if Instant::now() >= deadline {
                panic!("IPC socket was not created");
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let result = ipc_call(&endpoint_string, "devd.health", json!({}))
            .await
            .expect("health should pass");
        assert_eq!(result["ok"], true);

        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("daemon should stop after idle timeout")
            .expect("join should pass")
            .expect("serve should exit cleanly");
        assert!(!endpoint.exists());
    }

    #[test]
    fn extracts_json_frames_from_mixed_cdc_line() {
        let mut input = vec![0xff, 0x00, 0x91, 0x92, 0x00, b'x'];
        input.extend_from_slice(
            br#"{"type":"log","level":"info","target":"usb_jsonl","message":"ok"}"#,
        );
        input.push(b'\n');

        let frames = extract_json_frames_from_cdc_line(&input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["type"], "log");
        assert_eq!(frames[0]["message"], "ok");
    }

    #[test]
    fn extracts_response_frame_when_binary_prefix_is_present() {
        let mut input = vec![0xff, 0x00, 0x31, 0x32, 0x00, b'a', b'b'];
        input.extend_from_slice(br#"{"id":"req-1","ok":true,"result":{"accepted":true}}"#);
        input.push(b'\n');

        let frames = extract_json_frames_from_cdc_line(&input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["id"], "req-1");
        assert_eq!(frames[0]["result"]["accepted"], true);
    }

    #[test]
    fn summarizes_raw_and_binary_cdc_lines() {
        let raw = summarize_cdc_line(b"boot: ready\r\n");
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].kind, SerialCdcTraceKind::Raw);
        assert_eq!(raw[0].payload, "boot: ready");

        let binary = summarize_cdc_line(&[0xff, 0x00, 0x91, 0x92, 0x00, b'\n']);
        assert_eq!(binary.len(), 1);
        assert_eq!(binary[0].kind, SerialCdcTraceKind::Defmt);
        assert!(binary[0].payload.contains("ff 00 91 92 00"));
    }

    #[test]
    fn web_mdns_service_info_uses_expected_shape() {
        let info =
            build_web_mdns_service_info("isohub-devd", 51200).expect("service info should build");

        assert_eq!(info.get_fullname(), "isohub-devd._isohub-devd._tcp.local.");
        assert_eq!(info.get_hostname(), "isohub-devd.local.");
        assert_eq!(info.get_port(), 51200);
        assert_eq!(info.get_property_val_str("app"), Some("isohub-devd"));
        assert_eq!(info.get_property_val_str("mode"), Some("web"));
        assert_eq!(info.get_property_val_str("api"), Some("/api/v1/bootstrap"));
        assert_eq!(info.get_property_val_str("version"), Some(release_version()));
    }
}
