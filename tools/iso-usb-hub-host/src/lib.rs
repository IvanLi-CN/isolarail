use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

pub const LOCAL_DEVICE_ID: &str = "local";
pub const SAMPLE_SNAPSHOT: &str = include_str!("../fixtures/snapshot-mixed.json");

#[derive(Debug)]
pub enum HostError {
    Io(io::Error),
    Json(serde_json::Error),
    NoSnapshot,
    NoSnapshotSource,
    BadRequest(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Json(err) => write!(f, "JSON error: {err}"),
            Self::NoSnapshot => write!(f, "no diag.snapshot JSON found"),
            Self::NoSnapshotSource => write!(
                f,
                "no snapshot source provided; pass --snapshot-file <path>, set ISO_USB_HUB_SNAPSHOT_FILE, or use --sample"
            ),
            Self::BadRequest(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<io::Error> for HostError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for HostError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type Result<T> = std::result::Result<T, HostError>;

pub fn sample_snapshot() -> Value {
    serde_json::from_str(SAMPLE_SNAPSHOT).expect("sample snapshot fixture is valid")
}

pub fn extract_snapshot_from_line(line: &str) -> Result<Option<Value>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Some(json_start) = trimmed.find('{') else {
        return Ok(None);
    };
    let prefix = &trimmed[..json_start];
    if !prefix.is_empty() && !prefix.contains("diag.snapshot") {
        return Ok(None);
    }
    serde_json::from_str(&trimmed[json_start..])
        .map(Some)
        .map_err(HostError::Json)
}

pub fn read_snapshot_from_reader<R: BufRead>(reader: R) -> Result<Value> {
    let mut last = None;
    for line in reader.lines() {
        match extract_snapshot_from_line(&line?) {
            Ok(Some(snapshot)) => last = Some(snapshot),
            Ok(None) => {}
            Err(HostError::Json(err)) if err.is_eof() => {}
            Err(err) => return Err(err),
        }
    }
    last.ok_or(HostError::NoSnapshot)
}

pub fn read_snapshot_file(path: impl AsRef<Path>) -> Result<Value> {
    let path = path.as_ref();
    let mut text = String::new();
    File::open(path)?.read_to_string(&mut text)?;
    if text.trim_start().starts_with('{') {
        return serde_json::from_str(&text).map_err(HostError::Json);
    }
    read_snapshot_from_reader(BufReader::new(text.as_bytes()))
}

pub fn read_snapshot_source(path: Option<&str>, sample: bool) -> Result<Value> {
    read_snapshot_source_with_env(
        path,
        std::env::var("ISO_USB_HUB_SNAPSHOT_FILE").ok(),
        sample,
    )
}

fn read_snapshot_source_with_env(
    path: Option<&str>,
    env_path: Option<String>,
    sample: bool,
) -> Result<Value> {
    if let Some(path) = path {
        read_snapshot_file(path)
    } else if let Some(path) = env_path {
        read_snapshot_file(path)
    } else if sample {
        Ok(sample_snapshot())
    } else {
        Err(HostError::NoSnapshotSource)
    }
}

pub fn device_list(snapshot: &Value) -> Value {
    json!([
        {
            "id": LOCAL_DEVICE_ID,
            "transport": "snapshot-log",
            "state": snapshot.pointer("/boot/outcome").and_then(Value::as_str).unwrap_or("unknown"),
            "firmware": snapshot.get("firmware").cloned().unwrap_or_else(|| json!({}))
        }
    ])
}

pub fn device_status(snapshot: &Value) -> Value {
    let ports = snapshot
        .get("ports")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let online_ports = ports
        .iter()
        .filter(|port| port.get("state").and_then(Value::as_str) == Some("online"))
        .count();
    let degraded_ports = ports
        .iter()
        .filter(|port| {
            matches!(
                port.get("state").and_then(Value::as_str),
                Some("offline" | "error" | "skipped")
            )
        })
        .count();
    json!({
        "id": LOCAL_DEVICE_ID,
        "schema": snapshot.get("schema").cloned().unwrap_or(Value::Null),
        "sequence": snapshot.get("sequence").cloned().unwrap_or(Value::Null),
        "uptime_ms": snapshot.get("uptime_ms").cloned().unwrap_or(Value::Null),
        "outcome": snapshot.pointer("/boot/outcome").and_then(Value::as_str).unwrap_or("unknown"),
        "power_ready": snapshot.pointer("/power_input/ready").and_then(Value::as_bool).unwrap_or(false),
        "sideband": snapshot.pointer("/sideband/state").and_then(Value::as_str).unwrap_or("unknown"),
        "front_panel": snapshot.pointer("/front_panel/state").and_then(Value::as_str).unwrap_or("unknown"),
        "ports": {
            "total": ports.len(),
            "online": online_ports,
            "degraded": degraded_ports
        }
    })
}

pub fn format_status(snapshot: &Value) -> String {
    let status = device_status(snapshot);
    format!(
        "device={id} outcome={outcome} power_ready={power} sideband={sideband} front_panel={front} ports={online}/{total} online",
        id = status["id"].as_str().unwrap_or(LOCAL_DEVICE_ID),
        outcome = status["outcome"].as_str().unwrap_or("unknown"),
        power = status["power_ready"].as_bool().unwrap_or(false),
        sideband = status["sideband"].as_str().unwrap_or("unknown"),
        front = status["front_panel"].as_str().unwrap_or("unknown"),
        online = status["ports"]["online"].as_u64().unwrap_or(0),
        total = status["ports"]["total"].as_u64().unwrap_or(0)
    )
}

pub fn format_snapshot_tree(snapshot: &Value) -> String {
    let mut out = String::new();
    out.push_str(&format_status(snapshot));
    out.push('\n');
    out.push_str("hardware:\n");
    for key in ["power_input", "sideband", "front_panel", "fan"] {
        let state = snapshot
            .pointer(&format!("/{key}/state"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let present = snapshot
            .pointer(&format!("/{key}/present"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        out.push_str(&format!("  {key}: state={state} present={present}\n"));
    }
    if let Some(ports) = snapshot.get("ports").and_then(Value::as_array) {
        for port in ports {
            let idx = port.get("index").and_then(Value::as_u64).unwrap_or(0);
            let state = port
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let vbus = port
                .pointer("/telemetry/vbus_mv")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let current = port
                .pointer("/telemetry/current_ma")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let ina = port
                .pointer("/sensors/ina226/present")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let tmp = port
                .pointer("/sensors/tmp112/present")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            out.push_str(&format!(
                "  port{idx}: state={state} ina226={ina} tmp112={tmp} vbus={vbus}mV current={current}mA\n"
            ));
        }
    }
    out
}

pub fn http_response(method: &str, path: &str, snapshot: &Value) -> (u16, &'static str, String) {
    if method != "GET" {
        return (
            405,
            "application/json",
            json!({"error":"method_not_allowed"}).to_string(),
        );
    }
    let path = path.split('?').next().unwrap_or(path);
    match path {
        "/healthz" => (200, "application/json", json!({"ok": true}).to_string()),
        "/api/v1/devices" => (200, "application/json", device_list(snapshot).to_string()),
        "/api/v1/devices/local/status" => {
            (200, "application/json", device_status(snapshot).to_string())
        }
        "/api/v1/devices/local/diag-snapshot" => (
            200,
            "application/json",
            serde_json::to_string(snapshot).unwrap(),
        ),
        _ => (
            404,
            "application/json",
            json!({"error":"not_found"}).to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_snapshot_from_monitor_line() {
        let line =
            r#"1234 INFO diag.snapshot: {"schema":"iso-usb-hub.hardware.snapshot.v1","ports":[]}"#;
        let snapshot = extract_snapshot_from_line(line).unwrap().unwrap();
        assert_eq!(snapshot["schema"], "iso-usb-hub.hardware.snapshot.v1");
    }

    #[test]
    fn ignores_unrelated_log_lines() {
        assert!(extract_snapshot_from_line("INFO port.telemetry: p1=ok")
            .unwrap()
            .is_none());
    }

    #[test]
    fn keeps_last_complete_snapshot_when_log_tail_is_partial() {
        let valid =
            r#"INFO diag.snapshot: {"schema":"iso-usb-hub.hardware.snapshot.v1","sequence":7}"#;
        let partial = r#"INFO diag.snapshot: {"schema""#;
        let snapshot =
            read_snapshot_from_reader(std::io::Cursor::new(format!("{valid}\n{partial}\n")))
                .unwrap();
        assert_eq!(snapshot["sequence"], 7);
    }

    #[test]
    fn sample_snapshot_keeps_offline_nodes_non_fatal() {
        let snapshot = sample_snapshot();
        assert_eq!(snapshot["front_panel"]["state"], "offline");
        assert_eq!(snapshot["ports"][2]["state"], "offline");
        assert_eq!(device_status(&snapshot)["outcome"], "DEG");
    }

    #[test]
    fn snapshot_source_requires_explicit_input() {
        assert!(matches!(
            read_snapshot_source_with_env(None, None, false),
            Err(HostError::NoSnapshotSource)
        ));
    }

    #[test]
    fn http_bridge_routes_snapshot() {
        let snapshot = sample_snapshot();
        let (status, content_type, body) =
            http_response("GET", "/api/v1/devices/local/diag-snapshot", &snapshot);
        assert_eq!(status, 200);
        assert_eq!(content_type, "application/json");
        assert!(body.contains("iso-usb-hub.hardware.snapshot.v1"));
    }
}
