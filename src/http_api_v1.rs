#![allow(dead_code)]

use core::fmt::Write as _;

use heapless::String;

use crate::device_contract::{
    port_index_from_id, render_info_result, render_port_result, render_ports_result,
    render_wifi_result, RuntimeSnapshot, WifiSnapshot,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiRequest {
    Health,
    Info,
    Ports,
    Port { index: usize },
    PortPower { index: usize, enabled: bool },
    PortReplug { index: usize },
    Wifi,
    WifiSetForbidden,
    WifiClearForbidden,
    RebootForbidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiPendingAction {
    PortPower { index: usize, enabled: bool },
    PortReplug { index: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: &'static str,
    pub body: String<2048>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiOutcome {
    Response(ApiResponse),
    ResponseAndAction {
        response: ApiResponse,
        action: ApiPendingAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiErrorSpec {
    pub status: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub retryable: bool,
}

const BAD_REQUEST_UNKNOWN_ENDPOINT: ApiErrorSpec = ApiErrorSpec {
    status: "400 Bad Request",
    code: "bad_request",
    message: "unknown endpoint",
    retryable: false,
};

const BAD_REQUEST_ENABLED: ApiErrorSpec = ApiErrorSpec {
    status: "400 Bad Request",
    code: "bad_request",
    message: "missing or invalid enabled",
    retryable: false,
};

const INVALID_PORT: ApiErrorSpec = ApiErrorSpec {
    status: "404 Not Found",
    code: "invalid_port",
    message: "invalid port",
    retryable: false,
};

const UNSAFE_WIFI_WRITE: ApiErrorSpec = ApiErrorSpec {
    status: "403 Forbidden",
    code: "unsafe_transport",
    message: "Wi-Fi configuration changes require Web Serial or Local USB",
    retryable: false,
};

const UNSAFE_REBOOT: ApiErrorSpec = ApiErrorSpec {
    status: "403 Forbidden",
    code: "unsafe_transport",
    message: "Reboot to apply Wi-Fi changes requires Web Serial or Local USB",
    retryable: false,
};

pub fn parse_request(method: &str, path_and_query: &str) -> Result<ApiRequest, ApiErrorSpec> {
    let (path, query) = split_path_and_query(path_and_query);
    match (method, path) {
        ("GET", "/api/v1/health") => Ok(ApiRequest::Health),
        ("GET", "/api/v1/info") => Ok(ApiRequest::Info),
        ("GET", "/api/v1/ports") => Ok(ApiRequest::Ports),
        ("GET", "/api/v1/wifi") => Ok(ApiRequest::Wifi),
        ("POST", "/api/v1/wifi/set") => Err(UNSAFE_WIFI_WRITE),
        ("POST", "/api/v1/wifi/clear") => Err(UNSAFE_WIFI_WRITE),
        ("POST", "/api/v1/reboot") => Err(UNSAFE_REBOOT),
        _ => parse_port_request(method, path, query),
    }
}

pub fn render_health_json() -> &'static str {
    "{\"ok\":true}"
}

pub fn render_info_json(snapshot: RuntimeSnapshot, version: &str) -> String<1536> {
    render_info_result(snapshot, version)
}

pub fn render_ports_json(snapshot: RuntimeSnapshot) -> String<2048> {
    render_ports_result(snapshot)
}

pub fn render_port_json(snapshot: RuntimeSnapshot, index: usize) -> Option<String<768>> {
    render_port_result(snapshot, index)
}

pub fn render_wifi_json(wifi: WifiSnapshot) -> String<512> {
    render_wifi_result(wifi)
}

pub fn render_port_power_json(enabled: bool) -> String<128> {
    let mut out = String::<128>::new();
    let _ = write!(
        out,
        "{{\"accepted\":true,\"power_enabled\":{}}}",
        if enabled { "true" } else { "false" }
    );
    out
}

pub fn render_port_replug_json() -> &'static str {
    "{\"accepted\":true,\"mode\":\"power_cycle\"}"
}

pub fn render_error_json(error: ApiErrorSpec) -> String<256> {
    let mut out = String::<256>::new();
    let _ = write!(
        out,
        "{{\"error\":{{\"code\":\"{}\",\"message\":\"{}\",\"retryable\":{}}}}}",
        error.code,
        error.message,
        if error.retryable { "true" } else { "false" }
    );
    out
}

pub fn handle_request(
    method: &str,
    path_and_query: &str,
    snapshot: RuntimeSnapshot,
    version: &str,
) -> ApiOutcome {
    match parse_request(method, path_and_query) {
        Ok(ApiRequest::Health) => {
            ApiOutcome::Response(response_static("200 OK", render_health_json()))
        }
        Ok(ApiRequest::Info) => ApiOutcome::Response(response_owned(
            "200 OK",
            render_info_result(snapshot, version),
        )),
        Ok(ApiRequest::Ports) => {
            ApiOutcome::Response(response_owned("200 OK", render_ports_result(snapshot)))
        }
        Ok(ApiRequest::Port { index }) => match render_port_result(snapshot, index) {
            Some(body) => ApiOutcome::Response(response_owned("200 OK", body)),
            None => ApiOutcome::Response(response_owned(
                INVALID_PORT.status,
                render_error_json(INVALID_PORT),
            )),
        },
        Ok(ApiRequest::PortPower { index, enabled }) => ApiOutcome::ResponseAndAction {
            response: response_owned("200 OK", render_port_power_json(enabled)),
            action: ApiPendingAction::PortPower { index, enabled },
        },
        Ok(ApiRequest::PortReplug { index }) => ApiOutcome::ResponseAndAction {
            response: response_static("202 Accepted", render_port_replug_json()),
            action: ApiPendingAction::PortReplug { index },
        },
        Ok(ApiRequest::Wifi) => {
            ApiOutcome::Response(response_owned("200 OK", render_wifi_result(snapshot.wifi)))
        }
        Ok(ApiRequest::WifiSetForbidden) | Ok(ApiRequest::WifiClearForbidden) => {
            ApiOutcome::Response(response_owned(
                UNSAFE_WIFI_WRITE.status,
                render_error_json(UNSAFE_WIFI_WRITE),
            ))
        }
        Ok(ApiRequest::RebootForbidden) => ApiOutcome::Response(response_owned(
            UNSAFE_REBOOT.status,
            render_error_json(UNSAFE_REBOOT),
        )),
        Err(error) => ApiOutcome::Response(response_owned(error.status, render_error_json(error))),
    }
}

fn response_static(status: &'static str, body: &'static str) -> ApiResponse {
    let mut owned = String::<2048>::new();
    let _ = owned.push_str(body);
    ApiResponse {
        status,
        body: owned,
    }
}

fn response_owned<const N: usize>(status: &'static str, body: String<N>) -> ApiResponse {
    let mut owned = String::<2048>::new();
    let _ = owned.push_str(body.as_str());
    ApiResponse {
        status,
        body: owned,
    }
}

fn parse_port_request(method: &str, path: &str, query: &str) -> Result<ApiRequest, ApiErrorSpec> {
    let Some(rest) = path.strip_prefix("/api/v1/ports/") else {
        return Err(BAD_REQUEST_UNKNOWN_ENDPOINT);
    };
    let (port_id, tail) = rest.split_once('/').unwrap_or((rest, ""));
    let Some(index) = port_index_from_id(port_id) else {
        return Err(INVALID_PORT);
    };

    match (method, tail) {
        ("GET", "") => Ok(ApiRequest::Port { index }),
        ("POST", "actions/replug") => Ok(ApiRequest::PortReplug { index }),
        ("POST", "power") => {
            let Some(enabled) = parse_enabled_query(query) else {
                return Err(BAD_REQUEST_ENABLED);
            };
            Ok(ApiRequest::PortPower { index, enabled })
        }
        _ => Err(BAD_REQUEST_UNKNOWN_ENDPOINT),
    }
}

fn split_path_and_query(path_and_query: &str) -> (&str, &str) {
    path_and_query
        .split_once('?')
        .unwrap_or((path_and_query, ""))
}

fn parse_enabled_query(query: &str) -> Option<bool> {
    for part in query.split('&') {
        let (key, value) = part.split_once('=')?;
        if key != "enabled" {
            continue;
        }
        return match value {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        };
    }
    None
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
                configured: true,
                psk_configured: true,
                state: WifiState::Connected,
                ipv4: Some([192, 168, 1, 20]),
                is_static: false,
                ssid: {
                    let mut ssid = [0u8; 32];
                    ssid[0] = b'L';
                    ssid[1] = b'a';
                    ssid[2] = b'b';
                    ssid
                },
                ssid_len: 3,
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
    fn parses_canonical_four_port_routes() {
        assert_eq!(
            parse_request("GET", "/api/v1/health"),
            Ok(ApiRequest::Health)
        );
        assert_eq!(parse_request("GET", "/api/v1/info"), Ok(ApiRequest::Info));
        assert_eq!(parse_request("GET", "/api/v1/ports"), Ok(ApiRequest::Ports));
        assert_eq!(
            parse_request("GET", "/api/v1/ports/port4"),
            Ok(ApiRequest::Port { index: 3 })
        );
        assert_eq!(
            parse_request("POST", "/api/v1/ports/port1/power?enabled=1"),
            Ok(ApiRequest::PortPower {
                index: 0,
                enabled: true
            })
        );
        assert_eq!(
            parse_request("POST", "/api/v1/ports/port3/actions/replug"),
            Ok(ApiRequest::PortReplug { index: 2 })
        );
    }

    #[test]
    fn rejects_legacy_or_invalid_routes() {
        assert_eq!(
            parse_request("GET", "/api/v1/ports/port_a"),
            Err(INVALID_PORT)
        );
        assert_eq!(
            parse_request("POST", "/api/v1/ports/port1/power"),
            Err(BAD_REQUEST_ENABLED)
        );
        assert_eq!(
            parse_request("POST", "/api/v1/wifi/set"),
            Err(UNSAFE_WIFI_WRITE)
        );
        assert_eq!(parse_request("POST", "/api/v1/reboot"), Err(UNSAFE_REBOOT));
    }

    #[test]
    fn renders_canonical_payloads() {
        let snapshot = sample_snapshot();
        let port4 = render_port_json(snapshot, 3).expect("port4");
        let wifi = render_wifi_json(snapshot.wifi);
        assert!(
            render_info_json(snapshot, "0.1.0").contains("\"firmware\":{\"name\":\"iso-usb-hub\"")
        );
        assert!(render_ports_json(snapshot).contains("\"portId\":\"port4\""));
        assert!(port4.contains("\"overcurrent\":true"));
        assert!(wifi.contains("\"address\":\"0x50\""));
        assert!(wifi.contains("\"ssid\":\"Lab\""));
        assert_eq!(render_health_json(), "{\"ok\":true}");
        assert_eq!(
            render_port_replug_json(),
            "{\"accepted\":true,\"mode\":\"power_cycle\"}"
        );
        assert_eq!(
            render_port_power_json(false).as_str(),
            "{\"accepted\":true,\"power_enabled\":false}"
        );
    }

    #[test]
    fn handle_request_returns_read_only_payloads() {
        let snapshot = sample_snapshot();
        let response = match handle_request("GET", "/api/v1/info", snapshot, "0.1.0") {
            ApiOutcome::Response(response) => response,
            other => panic!("unexpected outcome: {:?}", other),
        };
        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"hostname\":\"isohub-ccddee\""));

        let port = match handle_request("GET", "/api/v1/ports/port4", snapshot, "0.1.0") {
            ApiOutcome::Response(response) => response,
            other => panic!("unexpected outcome: {:?}", other),
        };
        assert_eq!(port.status, "200 OK");
        assert!(port.body.contains("\"portId\":\"port4\""));
    }

    #[test]
    fn handle_request_returns_action_plans_for_mutations() {
        let snapshot = sample_snapshot();
        let power = handle_request(
            "POST",
            "/api/v1/ports/port2/power?enabled=0",
            snapshot,
            "0.1.0",
        );
        match power {
            ApiOutcome::ResponseAndAction { response, action } => {
                assert_eq!(response.status, "200 OK");
                assert!(response.body.contains("\"accepted\":true"));
                assert_eq!(
                    action,
                    ApiPendingAction::PortPower {
                        index: 1,
                        enabled: false,
                    }
                );
            }
            other => panic!("unexpected outcome: {:?}", other),
        }

        let replug = handle_request(
            "POST",
            "/api/v1/ports/port3/actions/replug",
            snapshot,
            "0.1.0",
        );
        match replug {
            ApiOutcome::ResponseAndAction { response, action } => {
                assert_eq!(response.status, "202 Accepted");
                assert_eq!(action, ApiPendingAction::PortReplug { index: 2 });
            }
            other => panic!("unexpected outcome: {:?}", other),
        }
    }

    #[test]
    fn handle_request_returns_forbidden_write_errors() {
        let snapshot = sample_snapshot();
        let response = match handle_request("POST", "/api/v1/wifi/set", snapshot, "0.1.0") {
            ApiOutcome::Response(response) => response,
            other => panic!("unexpected outcome: {:?}", other),
        };
        assert_eq!(response.status, "403 Forbidden");
        assert!(response.body.contains("\"unsafe_transport\""));
    }
}
