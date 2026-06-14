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
    pub response: String<2048>,
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
) -> Result<UsbResponse, ProtocolError> {
    let (request, _used) = serde_json_core::de::from_str::<Request<'_>>(line)
        .map_err(|_| ProtocolError::InvalidJson)?;
    let id = request.id.ok_or(ProtocolError::MissingField)?;
    let method = request.method.ok_or(ProtocolError::MissingField)?;

    match method {
        "info" => Ok(UsbResponse {
            response: render_info_response(id, snapshot, version),
            log: None,
            action: UsbAction::None,
        }),
        "ports.get" => Ok(UsbResponse {
            response: render_ports_response(id, snapshot),
            log: None,
            action: UsbAction::None,
        }),
        "wifi.get" => Ok(UsbResponse {
            response: render_wifi_response(id, snapshot.wifi),
            log: None,
            action: UsbAction::None,
        }),
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
                    id,
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
                    id,
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
                    id,
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
                response: render_simple_ok(id, r#"{"accepted":true,"mode":"power_cycle"}"#),
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
                response: render_simple_ok(id, r#"{"accepted":true}"#),
                log: Some(log),
                action: UsbAction::Reboot,
            })
        }
        _ => Err(ProtocolError::UnsupportedMethod),
    }
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

fn render_simple_ok(id: &str, result_json: &str) -> String<2048> {
    let mut out = String::<2048>::new();
    let _ = write!(
        out,
        "{{\"id\":\"{}\",\"ok\":true,\"result\":{}}}",
        id, result_json
    );
    out
}

fn render_info_response(id: &str, snapshot: RuntimeSnapshot, version: &str) -> String<2048> {
    render_simple_ok(id, render_info_result(snapshot, version).as_str())
}

fn render_ports_response(id: &str, snapshot: RuntimeSnapshot) -> String<2048> {
    render_simple_ok(id, render_ports_result(snapshot).as_str())
}

fn render_wifi_response(id: &str, wifi: WifiSnapshot) -> String<2048> {
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
