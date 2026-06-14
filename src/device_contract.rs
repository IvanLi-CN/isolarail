use core::fmt::Write as _;

use heapless::String;

use crate::device_identity::{
    fqdn_from_hostname, hostname_from_short_id, mac_to_string, short_id_from_mac, DEVICE_VARIANT,
    FIRMWARE_NAME,
};

pub const WIFI_STORAGE_KIND: &str = "eeprom";
pub const WIFI_STORAGE_ADDRESS: &str = "0x50";

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiState {
    Idle,
    Connecting,
    Connected,
    Error,
}

impl WifiState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WifiSnapshot {
    pub configured: bool,
    pub psk_configured: bool,
    pub state: WifiState,
    pub ipv4: Option<[u8; 4]>,
    pub is_static: bool,
    pub ssid: [u8; 32],
    pub ssid_len: u8,
}

impl WifiSnapshot {
    pub const fn disconnected() -> Self {
        Self {
            configured: false,
            psk_configured: false,
            state: WifiState::Idle,
            ipv4: None,
            is_static: false,
            ssid: [0; 32],
            ssid_len: 0,
        }
    }

    pub fn ssid(&self) -> Option<&str> {
        if self.ssid_len == 0 {
            return None;
        }
        core::str::from_utf8(&self.ssid[..self.ssid_len as usize]).ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortTelemetryStatus {
    Ok,
    Off,
    NotInserted,
    Error,
    Overcurrent,
}

impl PortTelemetryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Off => "off",
            Self::NotInserted => "not_inserted",
            Self::Error => "error",
            Self::Overcurrent => "overrange",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortSnapshot {
    pub label: &'static str,
    pub status: PortTelemetryStatus,
    pub voltage_mv: u32,
    pub current_ma: u32,
    pub power_enabled: bool,
    pub data_connected: bool,
    pub replugging: bool,
    pub busy: bool,
    pub overcurrent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubSnapshot {
    pub upstream_connected: bool,
    pub isolated_usb_fault: bool,
    pub isolated_downstream_connected: bool,
    pub isolated_usb_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub mac: [u8; 6],
    pub uptime_ms: u64,
    pub wifi: WifiSnapshot,
    pub hub: HubSnapshot,
    pub ports: [PortSnapshot; 4],
}

pub const CANONICAL_PORT_IDS: [&str; 4] = ["port1", "port2", "port3", "port4"];

pub fn render_info_result(snapshot: RuntimeSnapshot, version: &str) -> String<1536> {
    let mut out = String::<1536>::new();
    let short_id = short_id_from_mac(snapshot.mac);
    let hostname = hostname_from_short_id(short_id.as_str());
    let fqdn = fqdn_from_hostname(hostname.as_str());
    let mac = mac_to_string(snapshot.mac);
    let _ = write!(
        out,
        "{{\"device\":{{\"device_id\":\"{}\",\"hostname\":\"{}\",\"fqdn\":\"{}\",\"mac\":\"{}\",\"variant\":\"{}\",\"firmware\":{{\"name\":\"{}\",\"version\":\"{}\"}},\"uptime_ms\":{},\"wifi\":{{\"state\":\"{}\",\"ipv4\":",
        short_id.as_str(),
        hostname.as_str(),
        fqdn.as_str(),
        mac.as_str(),
        DEVICE_VARIANT,
        FIRMWARE_NAME,
        version,
        snapshot.uptime_ms,
        snapshot.wifi.state.as_str(),
    );
    write_ipv4_or_null(&mut out, snapshot.wifi.ipv4);
    let _ = out.push_str(",\"is_static\":");
    let _ = out.push_str(bool_str(snapshot.wifi.is_static));
    let _ = out.push_str("}}}");
    out
}

pub fn render_ports_result(snapshot: RuntimeSnapshot) -> String<2048> {
    let mut out = String::<2048>::new();
    let _ = write!(
        out,
        "{{\"hub\":{{\"upstream_connected\":{},\"isolated_usb_fault\":{},\"isolated_downstream_connected\":{},\"isolated_usb_ready\":{}}},\"ports\":[",
        bool_str(snapshot.hub.upstream_connected),
        bool_str(snapshot.hub.isolated_usb_fault),
        bool_str(snapshot.hub.isolated_downstream_connected),
        bool_str(snapshot.hub.isolated_usb_ready),
    );
    for (index, port) in snapshot.ports.iter().enumerate() {
        if index > 0 {
            let _ = out.push(',');
        }
        write_port_json(&mut out, snapshot.uptime_ms, index, *port);
    }
    let _ = out.push_str("]}");
    out
}

#[allow(dead_code)]
pub fn render_port_result(snapshot: RuntimeSnapshot, index: usize) -> Option<String<768>> {
    let port = *snapshot.ports.get(index)?;
    let mut out = String::<768>::new();
    write_port_json(&mut out, snapshot.uptime_ms, index, port);
    Some(out)
}

pub fn render_wifi_result(wifi: WifiSnapshot) -> String<512> {
    let mut out = String::<512>::new();
    let _ = write!(
        out,
        "{{\"configured\":{},\"storage\":\"{}\",\"address\":\"{}\",\"ssid\":",
        bool_str(wifi.configured),
        WIFI_STORAGE_KIND,
        WIFI_STORAGE_ADDRESS,
    );
    write_json_string_or_null(&mut out, wifi.ssid());
    let _ = write!(
        out,
        ",\"psk_configured\":{},\"state\":\"{}\",\"ipv4\":",
        bool_str(wifi.psk_configured),
        wifi.state.as_str(),
    );
    write_ipv4_or_null(&mut out, wifi.ipv4);
    let _ = out.push_str(",\"is_static\":");
    let _ = out.push_str(bool_str(wifi.is_static));
    let _ = out.push('}');
    out
}

fn write_ipv4_or_null<const N: usize>(out: &mut String<N>, ipv4: Option<[u8; 4]>) {
    match ipv4 {
        None => {
            let _ = out.push_str("null");
        }
        Some([a, b, c, d]) => {
            let _ = write!(out, "\"{}.{}.{}.{}\"", a, b, c, d);
        }
    }
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn write_json_string_or_null<const N: usize>(out: &mut String<N>, value: Option<&str>) {
    let Some(value) = value else {
        let _ = out.push_str("null");
        return;
    };
    let _ = out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => {
                let _ = out.push_str("\\\"");
            }
            '\\' => {
                let _ = out.push_str("\\\\");
            }
            '\n' => {
                let _ = out.push_str("\\n");
            }
            '\r' => {
                let _ = out.push_str("\\r");
            }
            '\t' => {
                let _ = out.push_str("\\t");
            }
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => {
                let _ = out.push(c);
            }
        }
    }
    let _ = out.push('"');
}

pub fn port_index_from_id(port_id: &str) -> Option<usize> {
    match port_id {
        "port1" => Some(0),
        "port2" => Some(1),
        "port3" => Some(2),
        "port4" => Some(3),
        _ => None,
    }
}

fn write_port_json<const N: usize>(
    out: &mut String<N>,
    uptime_ms: u64,
    index: usize,
    port: PortSnapshot,
) {
    let Some(port_id) = CANONICAL_PORT_IDS.get(index) else {
        return;
    };
    let _ = write!(
        out,
        "{{\"portId\":\"{}\",\"label\":\"{}\",\"telemetry\":{{\"status\":\"{}\",\"voltage_mv\":{},\"current_ma\":{},\"power_mw\":{},\"sample_uptime_ms\":{}}},\"state\":{{\"power_enabled\":{},\"data_connected\":{},\"replugging\":{},\"busy\":{},\"overcurrent\":{}}},\"capabilities\":{{\"data_replug\":true,\"power_set\":true}}}}",
        port_id,
        port.label,
        port.status.as_str(),
        port.voltage_mv,
        port.current_ma,
        port.voltage_mv.saturating_mul(port.current_ma) / 1000,
        uptime_ms,
        bool_str(port.power_enabled),
        bool_str(port.data_connected),
        bool_str(port.replugging),
        bool_str(port.busy),
        bool_str(port.overcurrent),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> RuntimeSnapshot {
        RuntimeSnapshot {
            mac: [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
            uptime_ms: 1234,
            wifi: WifiSnapshot {
                configured: true,
                psk_configured: true,
                state: WifiState::Connected,
                ipv4: Some([192, 168, 1, 42]),
                is_static: false,
                ssid: {
                    let mut ssid = [0u8; 32];
                    ssid[0] = b'B';
                    ssid[1] = b'e';
                    ssid[2] = b'n';
                    ssid[3] = b'c';
                    ssid[4] = b'h';
                    ssid
                },
                ssid_len: 5,
            },
            hub: HubSnapshot {
                upstream_connected: true,
                isolated_usb_fault: false,
                isolated_downstream_connected: true,
                isolated_usb_ready: true,
            },
            ports: [
                PortSnapshot {
                    label: "Port 1",
                    status: PortTelemetryStatus::Ok,
                    voltage_mv: 5000,
                    current_ma: 100,
                    power_enabled: true,
                    data_connected: true,
                    replugging: false,
                    busy: false,
                    overcurrent: false,
                },
                PortSnapshot {
                    label: "Port 2",
                    status: PortTelemetryStatus::NotInserted,
                    voltage_mv: 0,
                    current_ma: 0,
                    power_enabled: false,
                    data_connected: false,
                    replugging: false,
                    busy: false,
                    overcurrent: false,
                },
                PortSnapshot {
                    label: "Port 3",
                    status: PortTelemetryStatus::Error,
                    voltage_mv: 0,
                    current_ma: 0,
                    power_enabled: false,
                    data_connected: false,
                    replugging: false,
                    busy: false,
                    overcurrent: false,
                },
                PortSnapshot {
                    label: "Port 4",
                    status: PortTelemetryStatus::Overcurrent,
                    voltage_mv: 5100,
                    current_ma: 120,
                    power_enabled: false,
                    data_connected: true,
                    replugging: true,
                    busy: true,
                    overcurrent: true,
                },
            ],
        }
    }

    #[test]
    fn info_result_uses_canonical_identity() {
        let body = render_info_result(sample_snapshot(), "0.1.0");
        assert!(body.as_str().contains("\"hostname\":\"isohub-ccddee\""));
        assert!(body
            .as_str()
            .contains("\"firmware\":{\"name\":\"iso-usb-hub\""));
        assert!(body.as_str().contains("\"variant\":\"v3\""));
    }

    #[test]
    fn ports_result_uses_four_port_ids() {
        let body = render_ports_result(sample_snapshot());
        assert!(body.as_str().contains("\"portId\":\"port1\""));
        assert!(body.as_str().contains("\"portId\":\"port4\""));
        assert!(body.as_str().contains("\"overcurrent\":true"));
        assert!(!body.as_str().contains("port_a"));
        assert!(!body.as_str().contains("port_c"));
    }

    #[test]
    fn wifi_result_reports_eeprom_storage() {
        let body = render_wifi_result(sample_snapshot().wifi);
        assert!(body.as_str().contains("\"storage\":\"eeprom\""));
        assert!(body.as_str().contains("\"address\":\"0x50\""));
        assert!(body.as_str().contains("\"ssid\":\"Bench\""));
        assert!(body.as_str().contains("\"state\":\"connected\""));
    }

    #[test]
    fn port_result_uses_single_canonical_port_shape() {
        let body = render_port_result(sample_snapshot(), 3).expect("port4 result");
        assert!(body.as_str().contains("\"portId\":\"port4\""));
        assert!(body.as_str().contains("\"label\":\"Port 4\""));
        assert!(body.as_str().contains("\"overcurrent\":true"));
    }

    #[test]
    fn canonical_port_id_parser_rejects_legacy_ids() {
        assert_eq!(port_index_from_id("port1"), Some(0));
        assert_eq!(port_index_from_id("port4"), Some(3));
        assert_eq!(port_index_from_id("port_a"), None);
        assert_eq!(port_index_from_id("USB-C"), None);
    }
}
