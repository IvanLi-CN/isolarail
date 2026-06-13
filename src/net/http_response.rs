fn write_port_json(body: &mut String, port_id: ApiPortId, label: &str, port: &ApiPortSnapshot) {
    let _ = core::write!(
        body,
        "{{\"portId\":\"{}\",\"label\":\"{}\",\"telemetry\":{{\"status\":\"{}\",\"voltage_mv\":",
        port_id.as_str(),
        label,
        port.telemetry.status.as_str(),
    );

    write_json_u32_or_null(body, port.telemetry.voltage_mv);
    let _ = body.push_str(",\"current_ma\":");
    write_json_u32_or_null(body, port.telemetry.current_ma);
    let _ = body.push_str(",\"power_mw\":");
    write_json_u32_or_null(body, port.telemetry.power_mw);
    let _ = core::write!(
        body,
        ",\"sample_uptime_ms\":{}}},\"state\":{{\"power_enabled\":{},\"data_connected\":{},\"replugging\":{},\"busy\":{}}},\"capabilities\":{{\"data_replug\":true,\"power_set\":true}}}}",
        port.telemetry.sample_uptime_ms,
        if port.state.power_enabled {
            "true"
        } else {
            "false"
        },
        if port.state.data_connected {
            "true"
        } else {
            "false"
        },
        if port.state.replugging {
            "true"
        } else {
            "false"
        },
        if port.state.busy { "true" } else { "false" },
    );
}

pub fn write_pd_diagnostics_json(body: &mut String, pd: &ApiPdSnapshot) {
    let _ = core::write!(
        body,
        "{{\"usb_c_power_enabled\":{},\"sw2303_i2c_allowed\":{},\"sw2303_profile_applied\":{},\"sw2303_stable_reads\":{},\"sw2303_error_latched\":{},\"tps_error_latched\":{},\"sw2303_readback_config\":",
        if pd.usb_c_power_enabled {
            "true"
        } else {
            "false"
        },
        if pd.sw2303_i2c_allowed {
            "true"
        } else {
            "false"
        },
        if pd.sw2303_profile_applied {
            "true"
        } else {
            "false"
        },
        pd.sw2303_stable_reads,
        if pd.sw2303_error_latched {
            "true"
        } else {
            "false"
        },
        if pd.tps_error_latched {
            "true"
        } else {
            "false"
        },
    );
    write_sw2303_readback_json(
        body,
        pd.sw2303_readback_config,
        pd.sw2303_readback_matches_config,
    );
    let _ = body.push_str(",\"sw2303_request\":{\"mv\":");
    write_json_u32_or_null(body, pd.sw2303_request_mv);
    let _ = body.push_str(",\"ma\":");
    write_json_u32_or_null(body, pd.sw2303_request_ma);
    let _ = body.push_str("},\"sw2303_last_valid_request\":{\"mv\":");
    write_json_u32_or_null(body, pd.sw2303_last_valid_mv);
    let _ = body.push_str(",\"ma\":");
    write_json_u32_or_null(body, pd.sw2303_last_valid_ma);
    let _ = body.push_str("},\"tps_setpoint\":{\"output_enabled\":");
    write_json_bool_or_null(body, pd.tps_setpoint_output_enabled);
    let _ = body.push_str(",\"mv\":");
    write_json_u32_or_null(body, pd.tps_setpoint_mv);
    let _ = body.push_str(",\"ilim_ma\":");
    write_json_u32_or_null(body, pd.tps_setpoint_ilim_ma);
    let _ = core::write!(
        body,
        "}},\"runtime_recovery_count\":{},\"sample_uptime_ms\":{}}}",
        pd.runtime_recovery_count,
        pd.sample_uptime_ms
    );
}

fn write_sw2303_readback_json(
    body: &mut String,
    readback: crate::power_config::Sw2303CapabilityReadback,
    matches_config: bool,
) {
    let _ = core::write!(
        body,
        "{{\"available\":{},\"matches_config\":{},\"power_watts\":",
        if readback.available { "true" } else { "false" },
        if matches_config { "true" } else { "false" },
    );
    write_json_u32_or_null(body, readback.power_watts.map(|v| v as u32));
    let _ = body.push_str(",\"protocols\":{\"pd\":");
    write_json_bool_or_null(body, readback.pd_enabled);
    let _ = body.push_str(",\"qc20\":");
    write_json_bool_or_null(body, readback.qc20_enabled);
    let _ = body.push_str(",\"qc30\":");
    write_json_bool_or_null(body, readback.qc30_enabled);
    let _ = body.push_str(",\"fcp\":");
    write_json_bool_or_null(body, readback.fcp_enabled);
    let _ = body.push_str(",\"afc\":");
    write_json_bool_or_null(body, readback.afc_enabled);
    let _ = body.push_str(",\"scp\":");
    write_json_bool_or_null(body, readback.scp_enabled);
    let _ = body.push_str(",\"pe20\":");
    write_json_bool_or_null(body, readback.pe20_enabled);
    let _ = body.push_str(",\"bc12\":");
    write_json_bool_or_null(body, readback.bc12_enabled);
    let _ = body.push_str(",\"sfcp\":");
    write_json_bool_or_null(body, readback.sfcp_enabled);
    let _ = body.push_str("},\"pd\":{\"pps\":");
    write_json_bool_or_null(body, readback.pps_enabled);
    let _ = body.push_str(",\"fixed_voltages_mv\":[");
    write_fixed_voltage_json(body, readback.fixed_9v.unwrap_or(false), 9000);
    write_fixed_voltage_json(body, readback.fixed_12v.unwrap_or(false), 12000);
    write_fixed_voltage_json(body, readback.fixed_15v.unwrap_or(false), 15000);
    write_fixed_voltage_json(body, readback.fixed_20v.unwrap_or(false), 20000);
    let _ = body.push_str("]}}");
}

pub fn write_power_config_json(body: &mut String, power: &ApiPowerSnapshot) {
    let cfg = power.config;
    let _ = core::write!(
        body,
        "{{\"hardware\":\"{}\",\"persisted\":{},\"tps_mode\":\"{}\",\"capability\":{{\"profile\":\"full\",\"power_watts\":{},\"protocols\":{{\"pd\":{},\"qc20\":{},\"qc30\":{},\"fcp\":{},\"afc\":{},\"scp\":{},\"pe20\":{},\"bc12\":{},\"sfcp\":{}}},\"pd\":{{\"pps\":{},\"fixed_voltages_mv\":[",
        cfg.hardware.as_str(),
        if power.persisted { "true" } else { "false" },
        cfg.tps_mode.as_str(),
        cfg.capability.power_watts,
        cfg.capability.pd_enabled,
        cfg.capability.qc20_enabled,
        cfg.capability.qc30_enabled,
        cfg.capability.fcp_enabled,
        cfg.capability.afc_enabled,
        cfg.capability.scp_enabled,
        cfg.capability.pe20_enabled,
        cfg.capability.bc12_enabled,
        cfg.capability.sfcp_enabled,
        cfg.capability.pps_enabled,
    );
    write_fixed_voltage_json(body, cfg.capability.fixed_9v, 9000);
    write_fixed_voltage_json(body, cfg.capability.fixed_12v, 12000);
    write_fixed_voltage_json(body, cfg.capability.fixed_15v, 15000);
    write_fixed_voltage_json(body, cfg.capability.fixed_20v, 20000);
    let _ = core::write!(
        body,
        "]}}}},\"manual\":{{\"voltage_mv\":{},\"current_limit_ma\":{},\"usb_c_path_mode\":\"{}\",\"path_policy\":\"{}\"}},\"lock\":",
        cfg.manual.voltage_mv,
        cfg.manual.current_limit_ma,
        cfg.manual.usb_c_path_mode.as_str(),
        power
            .last_path_control
            .map(|control| control.as_str())
            .unwrap_or("unknown"),
    );
    match power.lock {
        Some(lock) => {
            let _ = core::write!(
                body,
                "{{\"owner\":{},\"expires_at_ms\":{}}}",
                lock.owner,
                lock.expires_at_ms
            );
        }
        None => {
            let _ = body.push_str("null");
        }
    }
    let _ = body.push_str("}");
}

fn write_fixed_voltage_json(body: &mut String, enabled: bool, mv: u32) {
    if !enabled {
        return;
    }
    if !body.ends_with('[') {
        let _ = body.push(',');
    }
    let _ = core::write!(body, "{}", mv);
}

fn write_json_bool_or_null(body: &mut String, v: Option<bool>) {
    match v {
        None => {
            let _ = body.push_str("null");
        }
        Some(true) => {
            let _ = body.push_str("true");
        }
        Some(false) => {
            let _ = body.push_str("false");
        }
    }
}

fn write_json_u32_or_null(body: &mut String, v: Option<u32>) {
    match v {
        None => {
            let _ = body.push_str("null");
        }
        Some(v) => {
            let _ = core::write!(body, "{}", v);
        }
    }
}

fn write_json_string(body: &mut String, value: &str) {
    let _ = body.push('"');
    for ch in value.chars() {
        match ch {
            '"' => {
                let _ = body.push_str("\\\"");
            }
            '\\' => {
                let _ = body.push_str("\\\\");
            }
            '\n' => {
                let _ = body.push_str("\\n");
            }
            '\r' => {
                let _ = body.push_str("\\r");
            }
            '\t' => {
                let _ = body.push_str("\\t");
            }
            ch if ch < ' ' => {
                let _ = core::write!(body, "\\u{:04x}", ch as u32);
            }
            ch => {
                let _ = body.push(ch);
            }
        }
    }
    let _ = body.push('"');
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiActionError {
    Busy,
}

const POWER_LOCK_TTL_MS: u64 = 15_000;

pub async fn try_set_power_lock(
    api_state: &'static ApiSharedMutex,
    owner: u32,
    acquire: bool,
) -> Result<(), ApiActionError> {
    let mut guard = api_state.lock().await;
    let now = uptime_ms();
    if let Some(lock) = guard.power.lock {
        if lock.expires_at_ms <= now {
            guard.power.lock = None;
        }
    }
    if acquire {
        if guard
            .power
            .lock
            .is_some_and(|lock| lock.owner != owner && lock.expires_at_ms > now)
        {
            return Err(ApiActionError::Busy);
        }
        guard.power.lock = Some(ApiPowerLock {
            owner,
            expires_at_ms: now + POWER_LOCK_TTL_MS,
        });
    } else if guard.power.lock.is_some_and(|lock| lock.owner == owner) {
        guard.power.lock = None;
    }
    Ok(())
}

pub async fn try_set_power_config(
    api_state: &'static ApiSharedMutex,
    command: ApiPowerConfigCommand,
    owner: Option<u32>,
) -> Result<(), ApiActionError> {
    let mut guard = api_state.lock().await;
    let now = uptime_ms();
    if let Some(lock) = guard.power.lock {
        if lock.expires_at_ms <= now {
            guard.power.lock = None;
        } else if owner != Some(lock.owner) {
            return Err(ApiActionError::Busy);
        }
    }
    if guard.pending.power_config.is_some() {
        return Err(ApiActionError::Busy);
    }
    crate::reset_power_config_result();
    guard.pending.power_config = Some(command);
    Ok(())
}

pub async fn try_set_action(
    api_state: &'static ApiSharedMutex,
    port_id: ApiPortId,
    action: ApiPortAction,
) -> Result<(), ApiActionError> {
    let mut guard = api_state.lock().await;
    let port = match port_id {
        ApiPortId::PortA => guard.ports.port_a,
        ApiPortId::PortC => guard.ports.port_c,
    };

    if port.state.busy
        || (port_id == ApiPortId::PortC && guard.pending.usb_c_downstream_route.is_some())
    {
        return Err(ApiActionError::Busy);
    }

    let slot = match port_id {
        ApiPortId::PortA => &mut guard.pending.port_a,
        ApiPortId::PortC => &mut guard.pending.port_c,
    };
    // If a previous action is still pending, treat as busy.
    if slot.is_some() {
        return Err(ApiActionError::Busy);
    }
    *slot = Some(action);
    Ok(())
}

pub async fn try_set_usb_c_downstream_route(
    api_state: &'static ApiSharedMutex,
    route: UsbCDownstreamRoute,
) -> Result<(), ApiActionError> {
    let mut guard = api_state.lock().await;
    if guard.ports.port_c.state.busy || guard.pending.usb_c_downstream_route.is_some() {
        return Err(ApiActionError::Busy);
    }
    crate::reset_usb_c_route_result();
    guard.pending.usb_c_downstream_route = Some(route);
    Ok(())
}

async fn write_preflight_response(
    socket: &mut TcpSocket<'_>,
    origin: Option<&str>,
    requested_headers: Option<&str>,
    request_private_network: bool,
    device_names: &'static DeviceNames,
) -> Result<(), embassy_net::tcp::Error> {
    let allow_origin = cors_allow_origin(origin);

    let mut headers = String::new();
    if let Some(origin) = allow_origin {
        let _ = core::write!(
            headers,
            "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\n",
            origin,
        );
    }

    let _ = headers.push_str("Access-Control-Allow-Methods: GET, POST, PUT, OPTIONS\r\n");
    let _ = core::write!(
        headers,
        "Access-Control-Allow-Headers: {}\r\n",
        requested_headers.unwrap_or("Content-Type")
    );

    if request_private_network {
        let mac = format_mac_lower(device_names.mac);
        let _ = headers.push_str("Access-Control-Allow-Private-Network: true\r\n");
        let _ = core::write!(
            headers,
            "Private-Network-Access-ID: {}\r\nPrivate-Network-Access-Name: {}\r\n",
            mac.as_str(),
            device_names.hostname.as_str(),
        );
    }

    write_http_response(socket, "204 No Content", None, headers.as_str(), "").await?;
    Ok(())
}

async fn write_api_error(
    socket: &mut TcpSocket<'_>,
    status: &str,
    allow_origin: Option<&str>,
    code: &str,
    message: &str,
    retryable: bool,
) -> Result<(), embassy_net::tcp::Error> {
    let mut body = String::new();
    let _ = core::write!(
        body,
        "{{\"error\":{{\"code\":\"{}\",\"message\":\"{}\",\"retryable\":{}}}}}",
        code,
        message,
        if retryable { "true" } else { "false" }
    );
    write_json_response(socket, status, allow_origin, body.as_str()).await
}

async fn write_json_response(
    socket: &mut TcpSocket<'_>,
    status: &str,
    allow_origin: Option<&str>,
    body: &str,
) -> Result<(), embassy_net::tcp::Error> {
    let mut extra_headers = String::new();
    let _ = extra_headers.push_str("Cache-Control: no-store\r\n");
    if let Some(origin) = allow_origin {
        let _ = core::write!(
            extra_headers,
            "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\n",
            origin,
        );
    }
    write_http_response(
        socket,
        status,
        Some("application/json; charset=utf-8"),
        extra_headers.as_str(),
        body,
    )
    .await
}

async fn write_plain_response(
    socket: &mut TcpSocket<'_>,
    status: &str,
    body: &str,
) -> Result<(), embassy_net::tcp::Error> {
    write_http_response(socket, status, Some("text/plain"), "", body).await
}

async fn write_http_response(
    socket: &mut TcpSocket<'_>,
    status: &str,
    content_type: Option<&str>,
    extra_headers: &str,
    body: &str,
) -> Result<(), embassy_net::tcp::Error> {
    let mut header = String::new();
    let _ = core::write!(header, "HTTP/1.1 {}\r\n", status);
    if let Some(ct) = content_type {
        let _ = core::write!(header, "Content-Type: {}\r\n", ct);
    }
    let _ = core::write!(header, "Content-Length: {}\r\n", body.as_bytes().len());
    let _ = header.push_str("Connection: close\r\n");
    let _ = header.push_str(extra_headers);
    let _ = header.push_str("\r\n");

    socket_write_all(socket, header.as_bytes()).await?;
    socket_write_all(socket, body.as_bytes()).await?;
    Ok(())
}

async fn socket_write_all(
    socket: &mut TcpSocket<'_>,
    mut buf: &[u8],
) -> Result<(), embassy_net::tcp::Error> {
    while !buf.is_empty() {
        let written = socket.write(buf).await?;
        if written == 0 {
            return Err(embassy_net::tcp::Error::ConnectionReset);
        }
        buf = &buf[written..];
    }
    Ok(())
}
