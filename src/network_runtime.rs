use alloc::string::String as AllocString;
use core::fmt::Write as _;

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{
    tcp::TcpSocket, Config as NetConfig, ConfigV4, DhcpConfig, Ipv4Address, Ipv4Cidr, Stack,
    StackResources, StaticConfigV4,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write as _;
use esp_hal::{peripherals::WIFI, rng::Rng, timer::timg::TimerGroup};
use esp_wifi::wifi::{self, ClientConfiguration, Configuration, WifiController, WifiDevice};
use heapless::String;
use heapless08::Vec;
use static_cell::StaticCell;

use crate::device_contract::{
    render_info_result, render_ports_result, render_wifi_result, RuntimeSnapshot, WifiState,
};
use crate::device_identity::{fqdn_from_hostname, hostname_from_short_id, short_id_from_mac};
use crate::http_api_v1::render_health_json;
use crate::mdns::{self, MdnsConfig};
use crate::provisioning::WifiCredentials;

const HTTP_PORT: u16 = 80;
const WIFI_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(5);
const WIFI_DHCP_POLL: Duration = Duration::from_millis(500);
const WIFI_DHCP_ATTEMPTS: u8 = 30;

static NET_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();
static WIFI_INIT: StaticCell<esp_wifi::EspWifiController<'static>> = StaticCell::new();
static NETWORK_STATE: NetworkStateMutex = Mutex::new(NetworkState::new());
static WIFI_APPLY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static SNAPSHOT_SIGNAL: Signal<CriticalSectionRawMutex, RuntimeSnapshot> = Signal::new();

pub type NetworkStateMutex = Mutex<CriticalSectionRawMutex, NetworkState>;

#[derive(Clone, Copy)]
pub struct NetworkState {
    pub wifi: crate::device_contract::WifiSnapshot,
}

impl NetworkState {
    const fn new() -> Self {
        Self {
            wifi: crate::device_contract::WifiSnapshot::disconnected(),
        }
    }
}

pub struct NetworkHandles {
    pub state: &'static NetworkStateMutex,
}

pub fn request_wifi_runtime_apply() {
    WIFI_APPLY_SIGNAL.signal(());
}

pub fn publish_snapshot(snapshot: RuntimeSnapshot) {
    SNAPSHOT_SIGNAL.signal(snapshot);
}

pub fn spawn(
    spawner: &Spawner,
    wifi: WIFI<'static>,
    timg1: esp_hal::peripherals::TIMG1<'static>,
    rng: esp_hal::peripherals::RNG<'static>,
    mac: [u8; 6],
    credentials: Option<WifiCredentials>,
) -> Option<NetworkHandles> {
    let mut rng = Rng::new(rng);
    let seed = ((rng.random() as u64) << 32) | rng.random() as u64;
    let init = match esp_wifi::init(TimerGroup::new(timg1).timer0, rng) {
        Ok(init) => init,
        Err(err) => {
            warn!("network.wifi: init failed err={:?}", err);
            return None;
        }
    };
    let init = WIFI_INIT.init(init);
    let (controller, interfaces) = match wifi::new(init, wifi) {
        Ok(parts) => parts,
        Err(err) => {
            warn!("network.wifi: controller init failed err={:?}", err);
            return None;
        }
    };

    let (net_cfg, initial_is_static) = build_net_config(credentials.as_ref());
    let resources = NET_RESOURCES.init(StackResources::<8>::new());
    let (stack, runner) = embassy_net::new(interfaces.sta, net_cfg, resources, seed);
    let names = NetworkNames::from_mac(mac);

    if credentials.is_none() {
        info!("network.wifi: credentials not configured; waiting for provisioning");
    }

    spawner.spawn(net_task(runner)).ok()?;
    spawner
        .spawn(wifi_task(
            controller,
            stack,
            &NETWORK_STATE,
            credentials,
            initial_is_static,
        ))
        .ok()?;
    spawner.spawn(http_task(stack)).ok()?;
    spawner
        .spawn(mdns::mdns_task(
            stack,
            MdnsConfig {
                hostname: names.hostname.clone(),
                hostname_fqdn: names.fqdn.clone(),
                instance_name: mdns::service_instance_name(names.hostname.as_str()),
                port: HTTP_PORT,
            },
        ))
        .ok()?;

    Some(NetworkHandles {
        state: &NETWORK_STATE,
    })
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn wifi_task(
    mut controller: WifiController<'static>,
    stack: Stack<'static>,
    state: &'static NetworkStateMutex,
    initial_credentials: Option<WifiCredentials>,
    initial_is_static: bool,
) {
    let mut active_credentials = initial_credentials;
    let mut is_static = initial_is_static;
    loop {
        let Some(credentials) = active_credentials else {
            update_wifi_state(state, None, WifiState::Idle, None, false).await;
            WIFI_APPLY_SIGNAL.wait().await;
            (active_credentials, is_static) = reload_wifi_config(stack).await;
            continue;
        };

        update_wifi_state(
            state,
            Some(&credentials),
            WifiState::Connecting,
            None,
            is_static,
        )
        .await;

        let conf = Configuration::Client(ClientConfiguration {
            ssid: AllocString::from(credentials.ssid()),
            password: AllocString::from(credentials.psk()),
            ..Default::default()
        });

        if let Err(err) = controller.set_configuration(&conf) {
            warn!("network.wifi: set_configuration failed err={:?}", err);
            update_wifi_state(state, Some(&credentials), WifiState::Error, None, is_static).await;
            wait_or_reload_credentials(stack, &mut active_credentials, &mut is_static).await;
            continue;
        }

        if let Err(err) = controller.start_async().await {
            warn!("network.wifi: start failed err={:?}", err);
            update_wifi_state(state, Some(&credentials), WifiState::Error, None, is_static).await;
            wait_or_reload_credentials(stack, &mut active_credentials, &mut is_static).await;
            continue;
        }

        match controller.connect_async().await {
            Ok(_) => {
                info!("network.wifi: sta connected; waiting for IPv4");
                let mut ipv4 = None;
                for _ in 0..WIFI_DHCP_ATTEMPTS {
                    if let Some(cfg) = stack.config_v4() {
                        ipv4 = Some(cfg.address.address().octets());
                        break;
                    }
                    match select(Timer::after(WIFI_DHCP_POLL), WIFI_APPLY_SIGNAL.wait()).await {
                        Either::First(()) => {}
                        Either::Second(()) => {
                            (active_credentials, is_static) = reload_wifi_config(stack).await;
                            let _ = controller.disconnect_async().await;
                            let _ = controller.stop_async().await;
                            continue;
                        }
                    }
                }
                if let Some(ip) = ipv4 {
                    info!(
                        "network.wifi: link up ip={}.{}.{}.{}",
                        ip[0], ip[1], ip[2], ip[3]
                    );
                    update_wifi_state(
                        state,
                        Some(&credentials),
                        WifiState::Connected,
                        Some(ip),
                        is_static,
                    )
                    .await;
                    match select(
                        controller.wait_for_event(wifi::WifiEvent::StaDisconnected),
                        WIFI_APPLY_SIGNAL.wait(),
                    )
                    .await
                    {
                        Either::First(_) => {
                            warn!("network.wifi: sta disconnected; retrying");
                            active_credentials = crate::wifi_credentials_cache().await;
                        }
                        Either::Second(()) => {
                            (active_credentials, is_static) = reload_wifi_config(stack).await;
                            let _ = controller.disconnect_async().await;
                        }
                    }
                    let _ = controller.stop_async().await;
                } else {
                    warn!("network.wifi: IPv4 not ready before timeout");
                    update_wifi_state(state, Some(&credentials), WifiState::Error, None, is_static)
                        .await;
                    let _ = controller.disconnect_async().await;
                    let _ = controller.stop_async().await;
                    wait_or_reload_credentials(stack, &mut active_credentials, &mut is_static)
                        .await;
                }
            }
            Err(err) => {
                warn!("network.wifi: connect failed err={:?}", err);
                update_wifi_state(state, Some(&credentials), WifiState::Error, None, is_static)
                    .await;
                let _ = controller.stop_async().await;
                wait_or_reload_credentials(stack, &mut active_credentials, &mut is_static).await;
            }
        }
    }
}

async fn wait_or_reload_credentials(
    stack: Stack<'static>,
    active_credentials: &mut Option<WifiCredentials>,
    is_static: &mut bool,
) {
    match select(
        Timer::after(WIFI_CONNECT_RETRY_DELAY),
        WIFI_APPLY_SIGNAL.wait(),
    )
    .await
    {
        Either::First(()) | Either::Second(()) => {
            *active_credentials = crate::wifi_credentials_cache().await;
            let (next_ipv4, next_is_static) = build_ipv4_config(active_credentials.as_ref());
            stack.set_config_v4(next_ipv4);
            *is_static = next_is_static;
        }
    }
}

async fn reload_wifi_config(stack: Stack<'static>) -> (Option<WifiCredentials>, bool) {
    let credentials = crate::wifi_credentials_cache().await;
    let (ipv4, is_static) = build_ipv4_config(credentials.as_ref());
    stack.set_config_v4(ipv4);
    (credentials, is_static)
}

async fn update_wifi_state(
    state: &'static NetworkStateMutex,
    credentials: Option<&WifiCredentials>,
    wifi_state: WifiState,
    ipv4: Option<[u8; 4]>,
    is_static: bool,
) {
    let mut guard = state.lock().await;
    guard.wifi = wifi_snapshot_from_credentials(credentials, wifi_state, ipv4, is_static);
}

#[embassy_executor::task]
async fn http_task(stack: Stack<'static>) {
    let mut latest: Option<RuntimeSnapshot> = None;
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 2048];
    loop {
        stack.wait_config_up().await;
        while let Some(snapshot) = SNAPSHOT_SIGNAL.try_take() {
            latest = Some(snapshot);
        }
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(embassy_time_net::Duration::from_secs(10)));
        match socket.accept(HTTP_PORT).await {
            Ok(()) => {
                if let Err(err) = handle_connection(&mut socket, latest).await {
                    warn!("network.http: connection error err={:?}", err);
                }
                socket.close();
                let _ = socket.flush().await;
            }
            Err(err) => {
                warn!("network.http: accept failed err={:?}", err);
                Timer::after(Duration::from_millis(200)).await;
            }
        }
    }
}

async fn handle_connection(
    socket: &mut TcpSocket<'_>,
    snapshot: Option<RuntimeSnapshot>,
) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = [0u8; 1024];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request = core::str::from_utf8(&buf[..n]).unwrap_or("");
    let line = request.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let origin = header_value(request, "Origin");

    if method == "OPTIONS" {
        write_preflight(socket, origin).await?;
        return Ok(());
    }
    if method != "GET" {
        write_json(socket, "405 Method Not Allowed", origin, "{\"error\":{\"code\":\"method_not_allowed\",\"message\":\"method not allowed\",\"retryable\":false}}").await?;
        return Ok(());
    }

    let path_only = path.split('?').next().unwrap_or(path);
    match path_only {
        "/" | "/api/v1/health" => {
            write_json(socket, "200 OK", origin, render_health_json()).await?;
        }
        "/api/v1/info" => match snapshot {
            Some(snapshot) => {
                let body = render_info_result(snapshot, env!("CARGO_PKG_VERSION"));
                write_json(socket, "200 OK", origin, body.as_str()).await?;
            }
            None => {
                write_json(socket, "503 Service Unavailable", origin, "{\"error\":{\"code\":\"not_ready\",\"message\":\"runtime snapshot is not ready\",\"retryable\":true}}").await?;
            }
        },
        "/api/v1/ports" => match snapshot {
            Some(snapshot) => {
                let body = render_ports_result(snapshot);
                write_json(socket, "200 OK", origin, body.as_str()).await?;
            }
            None => {
                write_json(socket, "503 Service Unavailable", origin, "{\"error\":{\"code\":\"not_ready\",\"message\":\"runtime snapshot is not ready\",\"retryable\":true}}").await?;
            }
        },
        "/api/v1/wifi" => {
            let wifi = current_wifi().await;
            let body = render_wifi_result(wifi);
            write_json(socket, "200 OK", origin, body.as_str()).await?;
        }
        _ => {
            write_json(socket, "404 Not Found", origin, "{\"error\":{\"code\":\"not_found\",\"message\":\"not found\",\"retryable\":false}}").await?;
        }
    }
    Ok(())
}

async fn current_wifi() -> crate::device_contract::WifiSnapshot {
    NETWORK_STATE.lock().await.wifi
}

async fn write_preflight(
    socket: &mut TcpSocket<'_>,
    origin: Option<&str>,
) -> Result<(), embassy_net::tcp::Error> {
    let mut response = String::<384>::new();
    let _ = write!(response, "HTTP/1.1 204 No Content\r\n");
    write_cors_headers(&mut response, origin);
    let _ = write!(
        response,
        "Access-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    socket.write_all(response.as_bytes()).await
}

async fn write_json(
    socket: &mut TcpSocket<'_>,
    status: &str,
    origin: Option<&str>,
    body: &str,
) -> Result<(), embassy_net::tcp::Error> {
    let mut header = String::<512>::new();
    let _ = write!(header, "HTTP/1.1 {}\r\n", status);
    write_cors_headers(&mut header, origin);
    let _ = write!(
        header,
        "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(header.as_bytes()).await?;
    socket.write_all(body.as_bytes()).await
}

fn write_cors_headers<const N: usize>(out: &mut String<N>, origin: Option<&str>) {
    if let Some(origin) = origin.filter(|origin| is_allowed_origin(origin)) {
        let _ = write!(
            out,
            "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\n",
            origin
        );
        let _ = write!(out, "Access-Control-Allow-Private-Network: true\r\n");
    }
}

fn is_allowed_origin(origin: &str) -> bool {
    origin == "http://localhost"
        || origin.starts_with("http://localhost:")
        || origin == "http://127.0.0.1"
        || origin.starts_with("http://127.0.0.1:")
}

fn header_value<'a>(request: &'a str, key: &str) -> Option<&'a str> {
    for line in request.lines().skip(1) {
        let Some((candidate, value)) = line.split_once(':') else {
            continue;
        };
        if candidate.trim().eq_ignore_ascii_case(key) {
            return Some(value.trim());
        }
    }
    None
}

fn build_net_config(credentials: Option<&WifiCredentials>) -> (NetConfig, bool) {
    let (ipv4, is_static) = build_ipv4_config(credentials);
    let config = match ipv4 {
        ConfigV4::Static(config) => NetConfig::ipv4_static(config),
        ConfigV4::Dhcp(config) => NetConfig::dhcpv4(config),
        ConfigV4::None => NetConfig::dhcpv4(DhcpConfig::default()),
    };
    (config, is_static)
}

fn build_ipv4_config(credentials: Option<&WifiCredentials>) -> (ConfigV4, bool) {
    if let Some(static_ipv4) = credentials.and_then(|credentials| credentials.static_ipv4()) {
        let address = Ipv4Address::new(
            static_ipv4.address[0],
            static_ipv4.address[1],
            static_ipv4.address[2],
            static_ipv4.address[3],
        );
        let netmask = Ipv4Address::new(
            static_ipv4.netmask[0],
            static_ipv4.netmask[1],
            static_ipv4.netmask[2],
            static_ipv4.netmask[3],
        );
        let gateway = Ipv4Address::new(
            static_ipv4.gateway[0],
            static_ipv4.gateway[1],
            static_ipv4.gateway[2],
            static_ipv4.gateway[3],
        );
        if let Some(prefix) = netmask_to_prefix(netmask) {
            let mut dns_servers: Vec<Ipv4Address, 3> = Vec::new();
            if let Some(dns) = static_ipv4.dns {
                let _ = dns_servers.push(Ipv4Address::new(dns[0], dns[1], dns[2], dns[3]));
            }
            return (
                ConfigV4::Static(StaticConfigV4 {
                    address: Ipv4Cidr::new(address, prefix),
                    gateway: Some(gateway),
                    dns_servers,
                }),
                true,
            );
        }
    }
    (ConfigV4::Dhcp(DhcpConfig::default()), false)
}

fn netmask_to_prefix(mask: Ipv4Address) -> Option<u8> {
    let value = u32::from_be_bytes(mask.octets());
    let prefix = value.count_ones() as u8;
    let reconstructed = if prefix == 0 {
        0
    } else {
        u32::MAX.checked_shl(32 - prefix as u32)?
    };
    (reconstructed == value).then_some(prefix)
}

fn wifi_snapshot_from_credentials(
    credentials: Option<&WifiCredentials>,
    state: WifiState,
    ipv4: Option<[u8; 4]>,
    is_static: bool,
) -> crate::device_contract::WifiSnapshot {
    let Some(credentials) = credentials else {
        return crate::device_contract::WifiSnapshot::disconnected();
    };
    let mut ssid = [0u8; 32];
    let ssid_bytes = credentials.ssid().as_bytes();
    ssid[..ssid_bytes.len()].copy_from_slice(ssid_bytes);
    crate::device_contract::WifiSnapshot {
        configured: true,
        psk_configured: credentials.psk_configured(),
        state,
        ipv4,
        is_static,
        ssid,
        ssid_len: ssid_bytes.len() as u8,
    }
}

struct NetworkNames {
    hostname: String<32>,
    fqdn: String<48>,
}

impl NetworkNames {
    fn from_mac(mac: [u8; 6]) -> Self {
        let short_id = short_id_from_mac(mac);
        let hostname = hostname_from_short_id(short_id.as_str());
        let fqdn = fqdn_from_hostname(hostname.as_str());
        Self { hostname, fqdn }
    }
}
