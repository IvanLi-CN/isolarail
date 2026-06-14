//! Legacy network skeleton kept only as a migration reference.
//!
//! This module is intentionally not wired into `main.rs`.
//! It still carries the pre-alignment dual-port (`port_a` / `port_c`) model and
//! historical USB-C route/power-config semantics from an earlier product shape.
//! Do not treat it as the current IsoHub control-plane implementation.
//! The active owner-facing device contract is defined by:
//! - `src/device_identity.rs`
//! - `src/usb_jsonl.rs`
//! - `docs/specs/pw97u-control-plane-alignment/SPEC.md`
//!
//! Any future firmware HTTP/Wi-Fi implementation must be rebuilt around
//! `port1..port4`, `firmware.name="iso-usb-hub"`, `hostname=isohub-<shortid>`,
//! and the four-port V3 hardware model instead of reviving the dual-port API
//! shapes preserved here.

#![allow(dead_code)]

use alloc::string::String;
use core::fmt::Write as _;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{
    Config as NetConfig, DhcpConfig, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4,
    tcp::TcpSocket,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::WIFI, rng::Rng, time::Instant as HalInstant};
use esp_radio::{
    Controller as RadioController, init as radio_init,
    wifi::{self, ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent},
};
use heapless::{String as HString, Vec};
use crate::power_config::{
    ManualTpsConfig, ManualUsbCPathMode, PowerConfig, Sw2303CapabilityReadback, TpsMode,
    UsbCCapabilityConfig,
};
use crate::provisioning::{
    DEFAULT_USB_C_DOWNSTREAM_ROUTE, UsbCDownstreamRoute, WifiCredentials,
};
use crate::release_version;
use static_cell::StaticCell;

use crate::mdns;
use crate::mdns::MdnsConfig;
#[cfg(feature = "net_http")]
const WIFI_HOSTNAME: Option<&str> = option_env!("USB_HUB_WIFI_HOSTNAME");
#[cfg(feature = "net_http")]
const WIFI_STATIC_IP: Option<&str> = option_env!("USB_HUB_WIFI_STATIC_IP");
#[cfg(feature = "net_http")]
const WIFI_NETMASK: Option<&str> = option_env!("USB_HUB_WIFI_NETMASK");
#[cfg(feature = "net_http")]
const WIFI_GATEWAY: Option<&str> = option_env!("USB_HUB_WIFI_GATEWAY");
#[cfg(feature = "net_http")]
const WIFI_DNS: Option<&str> = option_env!("USB_HUB_WIFI_DNS");

pub struct NetHandles {
    pub device_names: &'static DeviceNames,
    pub wifi_state: &'static WifiStateMutex,
}

#[derive(Clone)]
pub struct DeviceNames {
    pub mac: [u8; 6],
    pub short_id: HString<6>,
    pub hostname: HString<32>,
    pub hostname_fqdn: HString<48>,
}

/// Shared Wi‑Fi/IPv4 state for UI + HTTP APIs.
#[derive(Clone, Copy, Debug)]
pub enum WifiConnectionState {
    Idle,
    Connecting,
    Connected,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub enum WifiErrorKind {
    ConnectFailed,
    DhcpTimeout,
    LinkLost,
}

#[derive(Clone, Copy, Debug)]
pub struct WifiState {
    pub state: WifiConnectionState,
    pub ipv4: Option<Ipv4Address>,
    pub gateway: Option<Ipv4Address>,
    pub is_static: bool,
    pub last_error: Option<WifiErrorKind>,
    pub mac: Option<[u8; 6]>,
}

impl WifiState {
    const fn new() -> Self {
        Self {
            state: WifiConnectionState::Idle,
            ipv4: None,
            gateway: None,
            is_static: false,
            last_error: None,
            mac: None,
        }
    }
}

pub type WifiStateMutex = Mutex<CriticalSectionRawMutex, WifiState>;

static WIFI_STATE_CELL: StaticCell<WifiStateMutex> = StaticCell::new();
static DEVICE_NAMES_CELL: StaticCell<DeviceNames> = StaticCell::new();
static RADIO_CONTROLLER: StaticCell<RadioController<'static>> = StaticCell::new();
static NET_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();
static API_STATE_CELL: StaticCell<ApiSharedMutex> = StaticCell::new();
static WIFI_APPLY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// --- HTTP API (Plan #0005) -------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPortId {
    PortA,
    PortC,
}

impl ApiPortId {
    pub const fn as_str(self) -> &'static str {
        match self {
            ApiPortId::PortA => "port_a",
            ApiPortId::PortC => "port_c",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiTelemetryStatus {
    Ok,
    Off,
    NotInserted,
    Error,
    Overrange,
}

impl ApiTelemetryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            ApiTelemetryStatus::Ok => "ok",
            ApiTelemetryStatus::Off => "off",
            ApiTelemetryStatus::NotInserted => "not_inserted",
            ApiTelemetryStatus::Error => "error",
            ApiTelemetryStatus::Overrange => "overrange",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPortTelemetry {
    pub status: ApiTelemetryStatus,
    pub voltage_mv: Option<u32>,
    pub current_ma: Option<u32>,
    pub power_mw: Option<u32>,
    pub sample_uptime_ms: u64,
}

impl ApiPortTelemetry {
    pub const fn unknown() -> Self {
        Self {
            status: ApiTelemetryStatus::Error,
            voltage_mv: None,
            current_ma: None,
            power_mw: None,
            sample_uptime_ms: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPortState {
    pub power_enabled: bool,
    pub data_connected: bool,
    pub replugging: bool,
    pub busy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiHubSnapshot {
    pub upstream_connected: bool,
    pub isolated_usb_fault: bool,
    pub isolated_downstream_connected: bool,
    pub isolated_usb_ready: bool,
    pub usb_c_downstream_route: UsbCDownstreamRoute,
    pub usb_c_downstream_persisted: bool,
}

impl ApiHubSnapshot {
    pub const fn unknown() -> Self {
        Self {
            upstream_connected: false,
            isolated_usb_fault: false,
            isolated_downstream_connected: false,
            isolated_usb_ready: false,
            usb_c_downstream_route: DEFAULT_USB_C_DOWNSTREAM_ROUTE,
            usb_c_downstream_persisted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPortSnapshot {
    pub telemetry: ApiPortTelemetry,
    pub state: ApiPortState,
}

impl ApiPortSnapshot {
    pub const fn unknown() -> Self {
        Self {
            telemetry: ApiPortTelemetry::unknown(),
            state: ApiPortState {
                power_enabled: false,
                data_connected: false,
                replugging: false,
                busy: false,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPortsSnapshot {
    pub port_a: ApiPortSnapshot,
    pub port_c: ApiPortSnapshot,
}

impl ApiPortsSnapshot {
    pub const fn unknown() -> Self {
        Self {
            port_a: ApiPortSnapshot::unknown(),
            port_c: ApiPortSnapshot::unknown(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPdSnapshot {
    pub usb_c_power_enabled: bool,
    pub sw2303_i2c_allowed: bool,
    pub sw2303_profile_applied: bool,
    pub sw2303_stable_reads: u32,
    pub sw2303_error_latched: bool,
    pub tps_error_latched: bool,
    pub sw2303_readback_config: Sw2303CapabilityReadback,
    pub sw2303_readback_matches_config: bool,
    pub sw2303_request_mv: Option<u32>,
    pub sw2303_request_ma: Option<u32>,
    pub sw2303_last_valid_mv: Option<u32>,
    pub sw2303_last_valid_ma: Option<u32>,
    pub tps_setpoint_output_enabled: Option<bool>,
    pub tps_setpoint_mv: Option<u32>,
    pub tps_setpoint_ilim_ma: Option<u32>,
    pub runtime_recovery_count: u32,
    pub sample_uptime_ms: u64,
}

impl ApiPdSnapshot {
    pub const fn unknown() -> Self {
        Self {
            usb_c_power_enabled: false,
            sw2303_i2c_allowed: false,
            sw2303_profile_applied: false,
            sw2303_stable_reads: 0,
            sw2303_error_latched: false,
            tps_error_latched: false,
            sw2303_readback_config: Sw2303CapabilityReadback::unavailable(),
            sw2303_readback_matches_config: false,
            sw2303_request_mv: None,
            sw2303_request_ma: None,
            sw2303_last_valid_mv: None,
            sw2303_last_valid_ma: None,
            tps_setpoint_output_enabled: None,
            tps_setpoint_mv: None,
            tps_setpoint_ilim_ma: None,
            runtime_recovery_count: 0,
            sample_uptime_ms: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPowerLock {
    pub owner: u32,
    pub expires_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPowerSnapshot {
    pub config: PowerConfig,
    pub persisted: bool,
    pub lock: Option<ApiPowerLock>,
    pub last_path_control: Option<crate::power_config::Sw2303PathControl>,
}

impl ApiPowerSnapshot {
    pub const fn unknown() -> Self {
        Self {
            config: PowerConfig::defaults(),
            persisted: false,
            lock: None,
            last_path_control: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPortAction {
    Replug,
    Power { enabled: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPowerConfigCommand {
    Set { config: PowerConfig },
    Defaults,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiPendingActions {
    pub port_a: Option<ApiPortAction>,
    pub port_c: Option<ApiPortAction>,
    pub usb_c_downstream_route: Option<UsbCDownstreamRoute>,
    pub power_config: Option<ApiPowerConfigCommand>,
}

impl ApiPendingActions {
    pub const fn empty() -> Self {
        Self {
            port_a: None,
            port_c: None,
            usb_c_downstream_route: None,
            power_config: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiSharedState {
    pub hub: ApiHubSnapshot,
    pub ports: ApiPortsSnapshot,
    pub pd: ApiPdSnapshot,
    pub power: ApiPowerSnapshot,
    pub pending: ApiPendingActions,
}

impl ApiSharedState {
    pub const fn new() -> Self {
        Self {
            hub: ApiHubSnapshot::unknown(),
            ports: ApiPortsSnapshot::unknown(),
            pd: ApiPdSnapshot::unknown(),
            power: ApiPowerSnapshot::unknown(),
            pending: ApiPendingActions::empty(),
        }
    }
}

pub type ApiSharedMutex = Mutex<CriticalSectionRawMutex, ApiSharedState>;

pub fn init_http_api_state() -> &'static ApiSharedMutex {
    API_STATE_CELL.init(Mutex::new(ApiSharedState::new()))
}

pub(crate) fn request_wifi_runtime_apply() {
    WIFI_APPLY_SIGNAL.signal(());
}

pub fn spawn_wifi_mdns_http(
    spawner: &Spawner,
    wifi_peripheral: WIFI<'static>,
    api_state: &'static ApiSharedMutex,
    credentials: Option<WifiCredentials>,
) -> Option<NetHandles> {
    let wifi_state = WIFI_STATE_CELL.init(Mutex::new(WifiState::new()));

    // Init radio driver (requires esp-rtos scheduler already started).
    let radio = match radio_init() {
        Ok(ctrl) => ctrl,
        Err(err) => {
            warn!(
                "Wi-Fi radio init failed; skipping Wi-Fi/mDNS/HTTP: {:?}",
                err
            );
            return None;
        }
    };
    let radio_ctrl = RADIO_CONTROLLER.init(radio);

    let (wifi_controller, wifi_interfaces) =
        match wifi::new(radio_ctrl, wifi_peripheral, Default::default()) {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    "Wi-Fi driver init failed; skipping Wi-Fi/mDNS/HTTP: {:?}",
                    err
                );
                return None;
            }
        };

    let wifi_device: WifiDevice<'static> = wifi_interfaces.sta;
    let wifi_mac = wifi_device.mac_address();
    let device_names = DEVICE_NAMES_CELL.init(derive_device_names(wifi_mac));

    if credentials.is_none() {
        info!("Wi-Fi credentials not configured in EEPROM; network services idle until configured");
    }

    let (net_cfg, is_static) = build_net_config_from_env(credentials.as_ref());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let resources = NET_RESOURCES.init(StackResources::<8>::new());
    let (stack, runner) = embassy_net::new(wifi_device, net_cfg, resources, seed);

    spawner
        .spawn(wifi_task(
            wifi_controller,
            stack,
            wifi_state,
            is_static,
            wifi_mac,
            credentials,
        ))
        .ok()?;

    spawner
        .spawn(http_task(stack, device_names, wifi_state, api_state))
        .ok()?;

    let mdns_cfg = MdnsConfig {
        hostname: device_names.hostname.clone(),
        hostname_fqdn: device_names.hostname_fqdn.clone(),
        instance_name: mdns::service_instance_name(device_names.hostname.as_str()),
        port: HTTP_PORT,
    };
    spawner.spawn(mdns::mdns_task(stack, mdns_cfg)).ok()?;

    spawner.spawn(net_task(runner)).ok()?;

    Some(NetHandles {
        device_names,
        wifi_state,
    })
}

pub fn format_network_toast_lines(
    short_id: Option<&str>,
    ip: Option<Ipv4Address>,
) -> [[u8; 20]; 3] {
    let mut lines = [[b' '; 20]; 3];

    // IMPORTANT: the toast UI uses a tiny fixed font that only supports:
    // digits, '.', '-', space, and a subset of uppercase letters.
    // Do NOT render arbitrary hostnames here (will show '?' for missing glyphs).

    // Line 0: device hint ("ID <short_id>") or a fallback.
    if let Some(id) = short_id.and_then(|v| {
        let s = v.trim();
        if s.is_empty() { None } else { Some(s) }
    }) {
        let mut out: HString<20> = HString::new();
        let _ = out.push_str("ID ");
        for ch in id.chars() {
            if out.len() >= 20 {
                break;
            }
            if ch.is_ascii_hexdigit() {
                let _ = out.push(ch.to_ascii_uppercase());
            }
        }
        let b = out.as_bytes();
        lines[0][..b.len()].copy_from_slice(b);
    } else {
        let b = b"NO WIFI";
        lines[0][..b.len()].copy_from_slice(b);
    }

    // Line 1: IP (full).
    match ip {
        None => {
            let b = b"NO IP";
            lines[1][..b.len()].copy_from_slice(b);
        }
        Some(ip) => {
            let o = ip.octets();
            let mut line1: HString<20> = HString::new();
            let _ = core::write!(line1, "IP {}.{}.{}.{}", o[0], o[1], o[2], o[3]);
            let b = line1.as_bytes();
            lines[1][..b.len()].copy_from_slice(b);
        }
    };

    lines
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn wifi_task(
    mut controller: WifiController<'static>,
    stack: Stack<'static>,
    state: &'static WifiStateMutex,
    initial_is_static_ip: bool,
    mac: [u8; 6],
    initial_credentials: Option<WifiCredentials>,
) {
    info!("Wi-Fi task starting (static_ip={})", initial_is_static_ip);
    let mut active_credentials = initial_credentials;

    'wifi: loop {
        let Some(credentials) = active_credentials else {
            if matches!(controller.is_started(), Ok(true)) {
                if let Err(err) = controller.stop_async().await {
                    warn!("Wi-Fi stop_async while unconfigured failed: {:?}", err);
                }
            }
            {
                let mut guard = state.lock().await;
                guard.state = WifiConnectionState::Idle;
                guard.ipv4 = None;
                guard.gateway = None;
                guard.is_static = false;
                guard.last_error = None;
                guard.mac = Some(mac);
            }
            WIFI_APPLY_SIGNAL.wait().await;
            active_credentials = crate::wifi_credentials_cache();
            continue;
        };

        let ssid = String::from(credentials.ssid());
        let password = String::from(credentials.psk());
        let (net_cfg, is_static_ip) = build_net_config_from_env(Some(&credentials));
        stack.set_config_v4(net_cfg.ipv4);

        {
            let mut guard = state.lock().await;
            guard.state = WifiConnectionState::Connecting;
            guard.ipv4 = None;
            guard.gateway = None;
            guard.last_error = None;
            guard.mac = Some(mac);
        }

        if matches!(controller.is_started(), Ok(true)) {
            if let Err(err) = controller.stop_async().await {
                warn!("Wi-Fi stop_async before reconfigure failed: {:?}", err);
                match select(
                    Timer::after(Duration::from_secs(2)),
                    WIFI_APPLY_SIGNAL.wait(),
                )
                .await
                {
                    Either::First(()) => {}
                    Either::Second(()) => active_credentials = crate::wifi_credentials_cache(),
                }
                continue;
            }
        }

        let client_config = ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(ssid.clone())
                .with_password(password.clone()),
        );

        if let Err(err) = controller.set_config(&client_config) {
            warn!("Wi-Fi set_config error: {:?}", err);
            {
                let mut guard = state.lock().await;
                guard.state = WifiConnectionState::Error;
                guard.last_error = Some(WifiErrorKind::ConnectFailed);
            }
            match select(
                Timer::after(Duration::from_secs(10)),
                WIFI_APPLY_SIGNAL.wait(),
            )
            .await
            {
                Either::First(()) => {}
                Either::Second(()) => active_credentials = crate::wifi_credentials_cache(),
            }
            continue;
        }

        info!("Starting Wi-Fi STA");
        if let Err(err) = controller.start_async().await {
            warn!("Wi-Fi start_async error: {:?}", err);
            {
                let mut guard = state.lock().await;
                guard.state = WifiConnectionState::Error;
                guard.last_error = Some(WifiErrorKind::ConnectFailed);
            }
            match select(
                Timer::after(Duration::from_secs(10)),
                WIFI_APPLY_SIGNAL.wait(),
            )
            .await
            {
                Either::First(()) => {}
                Either::Second(()) => active_credentials = crate::wifi_credentials_cache(),
            }
            continue;
        }

        info!("Connecting to Wi-Fi SSID=\"{}\"", ssid.as_str());
        match controller.connect_async().await {
            Ok(()) => {
                info!("Wi-Fi connect_async returned Ok; waiting for IPv4 config");

                let mut retries: u8 = 0;
                loop {
                    if stack.is_config_up() {
                        break;
                    }
                    if retries >= 30 {
                        warn!("Wi-Fi DHCP/static config not ready within timeout");
                        {
                            let mut guard = state.lock().await;
                            guard.state = WifiConnectionState::Error;
                            guard.last_error = Some(WifiErrorKind::DhcpTimeout);
                        }
                        break;
                    }
                    retries = retries.saturating_add(1);
                    match select(
                        Timer::after(Duration::from_millis(500)),
                        WIFI_APPLY_SIGNAL.wait(),
                    )
                    .await
                    {
                        Either::First(()) => {}
                        Either::Second(()) => {
                            active_credentials = crate::wifi_credentials_cache();
                            let _ = controller.disconnect_async().await;
                            continue 'wifi;
                        }
                    }
                }

                if !stack.is_config_up() {
                    match select(
                        Timer::after(Duration::from_secs(5)),
                        WIFI_APPLY_SIGNAL.wait(),
                    )
                    .await
                    {
                        Either::First(()) => {}
                        Either::Second(()) => {
                            active_credentials = crate::wifi_credentials_cache();
                            let _ = controller.disconnect_async().await;
                            continue 'wifi;
                        }
                    }
                    continue;
                }

                if let Some(cfg) = stack.config_v4() {
                    let ip = cfg.address.address();
                    let gw = cfg.gateway.unwrap_or(Ipv4Address::UNSPECIFIED);
                    info!("Wi-Fi link up: ip={} gw={}", ip, gw);
                    {
                        let mut guard = state.lock().await;
                        guard.state = WifiConnectionState::Connected;
                        guard.ipv4 = Some(ip);
                        guard.gateway = Some(gw);
                        guard.is_static = is_static_ip;
                        guard.last_error = None;
                        guard.mac = Some(mac);
                    }
                }

                match select(
                    controller.wait_for_event(WifiEvent::StaDisconnected),
                    WIFI_APPLY_SIGNAL.wait(),
                )
                .await
                {
                    Either::First(()) => {
                        warn!("Wi-Fi STA disconnected; will retry");
                        {
                            let mut guard = state.lock().await;
                            guard.state = WifiConnectionState::Error;
                            guard.last_error = Some(WifiErrorKind::LinkLost);
                        }
                        active_credentials = crate::wifi_credentials_cache();
                        match select(
                            Timer::after(Duration::from_secs(5)),
                            WIFI_APPLY_SIGNAL.wait(),
                        )
                        .await
                        {
                            Either::First(()) => {}
                            Either::Second(()) => {
                                active_credentials = crate::wifi_credentials_cache();
                                continue 'wifi;
                            }
                        }
                    }
                    Either::Second(()) => {
                        info!("Wi-Fi runtime configuration changed; reconnecting");
                        active_credentials = crate::wifi_credentials_cache();
                        if let Err(err) = controller.disconnect_async().await {
                            warn!(
                                "Wi-Fi disconnect_async during reconfigure failed: {:?}",
                                err
                            );
                        }
                    }
                }
            }
            Err(err) => {
                warn!("Wi-Fi connect_async error: {:?}", err);
                {
                    let mut guard = state.lock().await;
                    guard.state = WifiConnectionState::Error;
                    guard.last_error = Some(WifiErrorKind::ConnectFailed);
                }
                match select(
                    Timer::after(Duration::from_secs(10)),
                    WIFI_APPLY_SIGNAL.wait(),
                )
                .await
                {
                    Either::First(()) => active_credentials = crate::wifi_credentials_cache(),
                    Either::Second(()) => active_credentials = crate::wifi_credentials_cache(),
                }
            }
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}

include!("net/http.rs");

include!("net/names_config.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_hub_snapshot_defaults_to_upgrade_route() {
        assert_eq!(
            ApiHubSnapshot::unknown().usb_c_downstream_route,
            UsbCDownstreamRoute::Mcu
        );
    }
}
