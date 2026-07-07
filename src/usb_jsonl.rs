use core::fmt::Write as _;

use heapless::{String, Vec};
use serde::Deserialize;

use crate::device_contract::{
    port_index_from_id, render_info_result, render_ports_result, render_wifi_result,
    RuntimeSnapshot, WifiSnapshot,
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsbAction {
    None,
    Reboot,
    WifiSet {
        ssid: String<32>,
        psk: String<64>,
        psk_configured: bool,
    },
    WifiClear,
    PortPowerSet {
        index: usize,
        enabled: bool,
    },
    PortReplug {
        index: usize,
    },
}

pub struct UsbJsonlState<const N: usize> {
    buffer: Vec<u8, N>,
    in_frame: bool,
}

impl<const N: usize> UsbJsonlState<N> {
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            in_frame: false,
        }
    }

    pub fn push_byte(&mut self, byte: u8) -> Result<Option<String<N>>, ProtocolError> {
        match byte {
            b'\n' => {
                if !self.in_frame || self.buffer.is_empty() {
                    self.buffer.clear();
                    self.in_frame = false;
                    return Ok(None);
                }
                let mut line = String::<N>::new();
                let text = match core::str::from_utf8(self.buffer.as_slice()) {
                    Ok(text) => text,
                    Err(_) => {
                        self.buffer.clear();
                        self.in_frame = false;
                        return Err(ProtocolError::InvalidJson);
                    }
                };
                if line.push_str(text).is_err() {
                    self.buffer.clear();
                    self.in_frame = false;
                    return Err(ProtocolError::FrameTooLarge);
                }
                self.buffer.clear();
                self.in_frame = false;
                Ok(Some(line))
            }
            b'\r' => Ok(None),
            other => {
                if !self.in_frame {
                    if other != b'{' {
                        return Ok(None);
                    }
                    self.in_frame = true;
                }
                if self.buffer.push(other).is_err() {
                    self.buffer.clear();
                    self.in_frame = false;
                    return Err(ProtocolError::FrameTooLarge);
                }
                Ok(None)
            }
        }
    }
}

impl<const N: usize> Default for UsbJsonlState<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbResponse {
    pub response: String<16384>,
    pub log: Option<String<256>>,
    pub action: UsbAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidJson,
    MissingField,
    UnsupportedMethod,
    InvalidPort,
    FrameTooLarge,
    InvalidWifiInput,
    SnapshotUnavailable,
}

impl ProtocolError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid_json",
            Self::MissingField => "missing_field",
            Self::UnsupportedMethod => "unsupported_method",
            Self::InvalidPort => "invalid_port",
            Self::FrameTooLarge => "frame_too_large",
            Self::InvalidWifiInput => "invalid_wifi_input",
            Self::SnapshotUnavailable => "snapshot_unavailable",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidJson => "request frame is not valid JSON",
            Self::MissingField => "required request field is missing",
            Self::UnsupportedMethod => "request method is not supported by this firmware",
            Self::InvalidPort => "requested port is outside the supported port1..port4 range",
            Self::FrameTooLarge => "request frame exceeds the line buffer capacity",
            Self::InvalidWifiInput => "wifi credentials are invalid",
            Self::SnapshotUnavailable => "hardware snapshot is not available yet",
        }
    }
}

#[derive(Deserialize)]
struct Request<'a> {
    #[serde(borrow)]
    id: Option<&'a str>,
    #[serde(borrow)]
    method: Option<&'a str>,
    #[serde(default)]
    params: RequestParams<'a>,
}

#[derive(Deserialize)]
struct NumericIdRequest<'a> {
    id: Option<i64>,
    #[serde(borrow)]
    method: Option<&'a str>,
    #[serde(default)]
    params: RequestParams<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestId<'a> {
    String(&'a str),
    Number(i64),
}

impl RequestId<'_> {
    fn write_json<const N: usize>(&self, out: &mut String<N>) {
        match self {
            Self::String(id) => {
                let _ = out.push('"');
                write_json_escaped(out, id);
                let _ = out.push('"');
            }
            Self::Number(id) => {
                let _ = write!(out, "{id}");
            }
        }
    }
}

#[derive(Default, Deserialize)]
struct RequestParams<'a> {
    #[serde(borrow)]
    port: Option<&'a str>,
    enabled: Option<bool>,
    #[serde(borrow)]
    ssid: Option<&'a str>,
    #[serde(borrow)]
    psk: Option<&'a str>,
}

pub fn handle_request(
    line: &str,
    snapshot: RuntimeSnapshot,
    version: &str,
    hardware_snapshot: Option<&str>,
) -> Result<UsbResponse, ProtocolError> {
    let request = parse_request(line)?;
    match request.method {
        "info" => Ok(UsbResponse {
            response: render_info_response(request.id, snapshot, version),
            log: None,
            action: UsbAction::None,
        }),
        "ports.get" => Ok(UsbResponse {
            response: render_ports_response(request.id, snapshot),
            log: None,
            action: UsbAction::None,
        }),
        "wifi.get" => Ok(UsbResponse {
            response: render_wifi_response(request.id, snapshot.wifi),
            log: None,
            action: UsbAction::None,
        }),
        "hardware.snapshot" => {
            let snapshot_json = hardware_snapshot.ok_or(ProtocolError::SnapshotUnavailable)?;
            Ok(UsbResponse {
                response: render_simple_ok(request.id, snapshot_json),
                log: None,
                action: UsbAction::None,
            })
        }
        "wifi.set" => {
            let (ssid, psk) = validate_wifi_input(request.params.ssid, request.params.psk)?;
            let mut log = String::<256>::new();
            render_log_json(
                &mut log,
                LogLevel::Info,
                "usb_jsonl",
                "wifi credentials accepted over USB; runtime cache updated",
            );
            Ok(UsbResponse {
                response: render_simple_ok(
                    request.id,
                    r#"{"accepted":true,"reboot_required":false,"applied":false}"#,
                ),
                log: Some(log),
                action: UsbAction::WifiSet {
                    psk_configured: !psk.is_empty(),
                    ssid,
                    psk,
                },
            })
        }
        "wifi.clear" => {
            let mut log = String::<256>::new();
            render_log_json(
                &mut log,
                LogLevel::Info,
                "usb_jsonl",
                "wifi clear accepted over USB; runtime cache cleared",
            );
            Ok(UsbResponse {
                response: render_simple_ok(
                    request.id,
                    r#"{"accepted":true,"reboot_required":false,"applied":false}"#,
                ),
                log: Some(log),
                action: UsbAction::WifiClear,
            })
        }
        "port.power_set" => {
            let index = parse_port_index(request.params.port)?;
            let enabled = request.params.enabled.ok_or(ProtocolError::MissingField)?;
            let mut log = String::<256>::new();
            render_log_json(
                &mut log,
                LogLevel::Info,
                "usb_jsonl",
                if enabled {
                    "port power enabled"
                } else {
                    "port power disabled"
                },
            );
            Ok(UsbResponse {
                response: render_simple_ok(
                    request.id,
                    if enabled {
                        r#"{"accepted":true,"power_enabled":true}"#
                    } else {
                        r#"{"accepted":true,"power_enabled":false}"#
                    },
                ),
                log: Some(log),
                action: UsbAction::PortPowerSet { index, enabled },
            })
        }
        "port.replug" => {
            let index = parse_port_index(request.params.port)?;
            let mut log = String::<256>::new();
            render_log_json(
                &mut log,
                LogLevel::Info,
                "usb_jsonl",
                "port replug requested as power-cycle",
            );
            Ok(UsbResponse {
                response: render_simple_ok(request.id, r#"{"accepted":true,"mode":"power_cycle"}"#),
                log: Some(log),
                action: UsbAction::PortReplug { index },
            })
        }
        "reboot" => {
            let mut log = String::<256>::new();
            render_log_json(
                &mut log,
                LogLevel::Warn,
                "usb_jsonl",
                "software reboot accepted over USB",
            );
            Ok(UsbResponse {
                response: render_simple_ok(request.id, r#"{"accepted":true}"#),
                log: Some(log),
                action: UsbAction::Reboot,
            })
        }
        _ => Err(ProtocolError::UnsupportedMethod),
    }
}

struct ParsedRequest<'a> {
    id: RequestId<'a>,
    method: &'a str,
    params: RequestParams<'a>,
}

fn parse_request(line: &str) -> Result<ParsedRequest<'_>, ProtocolError> {
    if let Ok((request, _used)) = serde_json_core::de::from_str::<Request<'_>>(line) {
        return Ok(ParsedRequest {
            id: RequestId::String(request.id.ok_or(ProtocolError::MissingField)?),
            method: request.method.ok_or(ProtocolError::MissingField)?,
            params: request.params,
        });
    }
    let (request, _used) = serde_json_core::de::from_str::<NumericIdRequest<'_>>(line)
        .map_err(|_| ProtocolError::InvalidJson)?;
    Ok(ParsedRequest {
        id: RequestId::Number(request.id.ok_or(ProtocolError::MissingField)?),
        method: request.method.ok_or(ProtocolError::MissingField)?,
        params: request.params,
    })
}

pub fn render_protocol_error(request_id: Option<&str>, error: ProtocolError) -> String<256> {
    let mut out = String::<256>::new();
    let _ = write!(
        out,
        "{{\"ok\":false,\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}}",
        error.code(),
        error.message()
    );
    if let Some(id) = request_id {
        let _ = write!(out, ",\"id\":\"{}\"", id);
    }
    let _ = out.push('}');
    out
}

pub fn render_log_json(out: &mut String<256>, level: LogLevel, target: &str, message: &str) {
    out.clear();
    let _ = write!(
        out,
        "{{\"type\":\"log\",\"level\":\"{}\",\"target\":\"",
        level.as_str()
    );
    write_json_escaped(out, target);
    let _ = out.push_str("\",\"message\":\"");
    write_json_escaped(out, message);
    let _ = out.push_str("\"}");
}

fn validate_wifi_input(
    ssid: Option<&str>,
    psk: Option<&str>,
) -> Result<(String<32>, String<64>), ProtocolError> {
    let ssid = ssid.ok_or(ProtocolError::MissingField)?;
    let psk = psk.ok_or(ProtocolError::MissingField)?;
    if ssid.is_empty() || ssid.len() > 32 || (!psk.is_empty() && psk.len() < 8) || psk.len() > 64 {
        return Err(ProtocolError::InvalidWifiInput);
    }
    let mut ssid_out = String::<32>::new();
    let mut psk_out = String::<64>::new();
    ssid_out
        .push_str(ssid)
        .map_err(|_| ProtocolError::InvalidWifiInput)?;
    psk_out
        .push_str(psk)
        .map_err(|_| ProtocolError::InvalidWifiInput)?;
    Ok((ssid_out, psk_out))
}

fn parse_port_index(port: Option<&str>) -> Result<usize, ProtocolError> {
    port_index_from_id(port.ok_or(ProtocolError::MissingField)?).ok_or(ProtocolError::InvalidPort)
}

fn render_simple_ok(id: RequestId<'_>, result_json: &str) -> String<16384> {
    let mut out = String::<16384>::new();
    let _ = out.push_str("{\"id\":");
    id.write_json(&mut out);
    let _ = write!(out, ",\"ok\":true,\"result\":{}}}", result_json);
    out
}

fn render_info_response(
    id: RequestId<'_>,
    snapshot: RuntimeSnapshot,
    version: &str,
) -> String<16384> {
    render_simple_ok(id, render_info_result(snapshot, version).as_str())
}

fn render_ports_response(id: RequestId<'_>, snapshot: RuntimeSnapshot) -> String<16384> {
    render_simple_ok(id, render_ports_result(snapshot).as_str())
}

fn render_wifi_response(id: RequestId<'_>, wifi: WifiSnapshot) -> String<16384> {
    render_simple_ok(id, render_wifi_result(wifi).as_str())
}

fn write_json_escaped<const N: usize>(out: &mut String<N>, value: &str) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_contract::{HubSnapshot, PortSnapshot, PortTelemetryStatus, WifiState};

    fn sample_snapshot() -> RuntimeSnapshot {
        RuntimeSnapshot {
            mac: [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
            uptime_ms: 42,
            wifi: WifiSnapshot {
                configured: false,
                psk_configured: false,
                state: WifiState::Idle,
                ipv4: None,
                is_static: false,
                ssid: [0u8; 32],
                ssid_len: 0,
            },
            hub: HubSnapshot {
                upstream_connected: true,
                isolated_usb_fault: false,
                isolated_downstream_connected: true,
                isolated_usb_ready: true,
            },
            ports: [PortSnapshot {
                label: "Port",
                status: PortTelemetryStatus::Ok,
                voltage_mv: 5000,
                current_ma: 0,
                power_enabled: true,
                data_connected: false,
                replugging: false,
                busy: false,
                overcurrent: false,
            }; 4],
        }
    }

    #[test]
    fn accepts_numeric_and_string_request_ids() {
        let numeric = handle_request(
            r#"{"id":7,"method":"info","params":{}}"#,
            sample_snapshot(),
            "0.1.0",
            None,
        )
        .expect("numeric id should parse");
        assert!(numeric.response.contains(r#""id":7"#));

        let string = handle_request(
            r#"{"id":"devd-7","method":"info","params":{}}"#,
            sample_snapshot(),
            "0.1.0",
            None,
        )
        .expect("string id should parse");
        assert!(string.response.contains(r#""id":"devd-7""#));
    }

    #[test]
    fn returns_cached_hardware_snapshot() {
        let response = handle_request(
            r#"{"id":"diag-1","method":"hardware.snapshot","params":{}}"#,
            sample_snapshot(),
            "0.1.0",
            Some(r#"{"schema":"isolarail.hardware.snapshot.v1","ports":[]}"#),
        )
        .expect("hardware snapshot should be served");
        assert!(response.response.contains(r#""id":"diag-1""#));
        assert!(response
            .response
            .contains(r#""schema":"isolarail.hardware.snapshot.v1""#));
    }
}
