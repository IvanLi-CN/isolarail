fn configured_usb_port_allowlist() -> Option<Vec<String>> {
    let raw = std::env::var("ISOHUB_USB_PORT").ok()?;
    let ports = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    (!ports.is_empty()).then_some(ports)
}

fn ensure_port_allowed(port_path: &str) -> anyhow::Result<()> {
    let Some(allowlist) = configured_usb_port_allowlist() else {
        return Ok(());
    };
    if allowlist.iter().any(|allowed| allowed == port_path) {
        return Ok(());
    }
    Err(anyhow!(
        "serial port {port_path} is not allowed by ISOHUB_USB_PORT"
    ))
}

async fn validate_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(catalog): Json<FirmwareCatalog>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let errors = validate_catalog_shape(&catalog);
    Json(json!({"ok": errors.is_empty(), "errors": errors})).into_response()
}

async fn usb_jsonl_request(
    state: &AppState,
    device_id: &str,
    method: &str,
    params: Option<Value>,
) -> anyhow::Result<Value> {
    usb_jsonl_request_with_exclusive(state, device_id, method, params, None).await
}

async fn usb_jsonl_request_with_exclusive(
    state: &AppState,
    device_id: &str,
    method: &str,
    params: Option<Value>,
    allowed_exclusive_reason: Option<&str>,
) -> anyhow::Result<Value> {
    let (port_path, request_id, port_lock) = {
        let mut inner = state.inner.lock().await;
        let device = inner
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow!("device not found"))?;
        let usb = device
            .usb
            .as_ref()
            .ok_or_else(|| anyhow!("device has no Local USB target"))?;
        if let Some(reason) = inner.exclusive_ports.get(&usb.port_path)
            && allowed_exclusive_reason != Some(reason.as_str())
        {
            return Err(anyhow!("device busy: {reason}"));
        }
        let port_path = usb.port_path.clone();
        let port_lock = inner
            .port_operation_locks
            .entry(port_path.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        (port_path, next_id(), port_lock)
    };
    let _port_guard = acquire_port_operation_lock(&port_path, port_lock).await?;

    let request = json!({
        "id": request_id,
        "method": method,
        "params": params.unwrap_or_else(|| json!({})),
    });
    push_trace(state, device_id, "tx", method, &request).await;
    let roundtrip =
        tokio::task::spawn_blocking(move || serial_jsonl_roundtrip(&port_path, request))
            .await
            .context("serial worker join")??;
    push_trace(state, device_id, "rx", method, &roundtrip.response).await;
    for trace in &roundtrip.traces {
        let level = trace
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("raw");
        let message = trace
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("USB CDC activity");
        push_trace(state, device_id, level, message, trace).await;
    }
    for frame in &roundtrip.logs {
        let level = frame
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info");
        let target = frame
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("usb_jsonl");
        let message = frame
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("USB CDC log");
        push_log(
            state,
            device_id,
            level,
            &format!("{target}: {message}"),
            frame,
        )
        .await;
    }
    Ok(roundtrip.response)
}

async fn acquire_port_operation_lock(
    port_path: &str,
    port_lock: Arc<Mutex<()>>,
) -> anyhow::Result<OwnedMutexGuard<()>> {
    acquire_port_operation_lock_with_timeout(
        port_path,
        port_lock,
        Duration::from_millis(SERIAL_TIMEOUT_MS),
    )
    .await
}

async fn acquire_port_operation_lock_with_timeout(
    port_path: &str,
    port_lock: Arc<Mutex<()>>,
    timeout: Duration,
) -> anyhow::Result<OwnedMutexGuard<()>> {
    tokio::time::timeout(timeout, port_lock.lock_owned()).await.map_err(|_| {
        anyhow!("device busy: another Local USB operation is still running on {port_path}")
    })
}

async fn device_usb_port_path(state: &AppState, device_id: &str) -> anyhow::Result<String> {
    let inner = state.inner.lock().await;
    Ok(inner
        .devices
        .get(device_id)
        .ok_or_else(|| anyhow!("device not found"))?
        .usb
        .as_ref()
        .ok_or_else(|| anyhow!("device has no Local USB target"))?
        .port_path
        .clone())
}

fn serial_timeout_ms_for_method(method: &str) -> u64 {
    match method {
        "power.config_set" | "power.config_defaults" => SERIAL_POWER_CONFIG_TIMEOUT_MS,
        _ => SERIAL_TIMEOUT_MS,
    }
}

struct SerialRoundtrip {
    response: Value,
    logs: Vec<Value>,
    traces: Vec<Value>,
}

fn serial_jsonl_roundtrip(port_path: &str, request: Value) -> anyhow::Result<SerialRoundtrip> {
    ensure_port_allowed(port_path)?;
    let mut port = serialport::new(port_path, SERIAL_BAUD)
        .timeout(Duration::from_millis(50))
        .open()
        .with_context(|| format!("open serial port {port_path}"))?;
    let mut line = serde_json::to_string(&request)?;
    line.push('\n');
    use std::io::{Read as _, Write as _};
    port.write_all(line.as_bytes()).context("serial write")?;
    port.flush().context("serial flush")?;

    let expected_id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let deadline = Instant::now() + Duration::from_millis(serial_timeout_ms_for_method(method));
    let mut raw = Vec::<u8>::new();
    let mut logs = Vec::<Value>::new();
    let mut traces = Vec::<Value>::new();
    let mut buf = [0_u8; 256];
    while Instant::now() < deadline {
        match port.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                while let Some(pos) = raw.iter().position(|byte| *byte == b'\n') {
                    let frame = raw.drain(..=pos).collect::<Vec<_>>();
                    for trace in summarize_cdc_line(&frame) {
                        if trace.kind == SerialCdcTraceKind::Json {
                            continue;
                        }
                        traces.push(json!({
                            "kind": match trace.kind {
                                SerialCdcTraceKind::Json => "json",
                                SerialCdcTraceKind::Raw => "raw",
                                SerialCdcTraceKind::Defmt => "defmt",
                                SerialCdcTraceKind::Ignored => "ignored",
                            },
                            "summary": trace.summary,
                            "payload": trace.payload,
                        }));
                    }
                    for value in extract_json_frames_from_cdc_line(&frame) {
                        if is_structured_log_frame(&value) {
                            logs.push(value.clone());
                        }
                        if expected_id.is_none() || value.get("id") == expected_id.as_ref() {
                            return Ok(SerialRoundtrip {
                                response: value,
                                logs,
                                traces,
                            });
                        }
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => return Err(err).context("serial read"),
        }
    }
    if !raw.is_empty() {
        traces.push(json!({
            "kind": "ignored",
            "summary": "serial response timed out with trailing bytes",
            "payload": hex_preview(&raw, 96),
        }));
    }
    if let Some(summary) = summarize_serial_timeout(&traces) {
        return Err(anyhow!(
            "serial response timed out after non-JSON USB CDC activity ({summary})"
        ));
    }
    Err(anyhow!("serial response timed out"))
}

fn summarize_serial_timeout(traces: &[Value]) -> Option<String> {
    let summary = traces
        .iter()
        .rev()
        .take(3)
        .filter_map(|trace| {
            let kind = trace.get("kind").and_then(Value::as_str)?;
            let summary = trace.get("summary").and_then(Value::as_str)?;
            Some(format!("{kind}: {summary}"))
        })
        .collect::<Vec<_>>();
    (!summary.is_empty()).then(|| summary.join("; "))
}

fn is_structured_log_frame(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("log")
}

async fn run_flash_request(
    state: &AppState,
    device_id: &str,
    req: FlashRequest,
) -> anyhow::Result<Value> {
    let catalog: FirmwareCatalog = serde_json::from_slice(&fs::read(&req.catalog_path)?)?;
    let errors = validate_catalog_shape(&catalog);
    if !errors.is_empty() {
        return Err(anyhow!("invalid firmware catalog: {}", errors.join(", ")));
    }
    let artifact = catalog
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == req.artifact_id)
        .ok_or_else(|| anyhow!("artifact not found: {}", req.artifact_id))?;
    let app_file = if req.first_time {
        artifact
            .files
            .iter()
            .find(|file| file.kind == "elf" || file.kind == "full_image")
            .ok_or_else(|| anyhow!("first-time flash requires an elf or full_image artifact"))?
    } else {
        artifact
            .files
            .iter()
            .find(|file| file.kind == "app_bin")
            .ok_or_else(|| anyhow!("normal flash requires an app_bin artifact"))?
    };
    verify_artifact_file(&req.catalog_path, app_file)?;

    if !req.real {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "artifact_id": artifact.artifact_id,
            "target": artifact.target,
            "file": app_file.path,
        }));
    }

    if req.first_time && !req.confirm_non_project_firmware {
        return Err(anyhow!(
            "first-time full flash may target download-mode or non-project firmware; pass explicit non-project firmware confirmation"
        ));
    }

    let port_path = {
        let inner = state.inner.lock().await;
        inner
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow!("device not found"))?
            .usb
            .as_ref()
            .ok_or_else(|| anyhow!("device has no Local USB target"))?
            .port_path
            .clone()
    };
    ensure_port_allowed(&port_path)?;

    if !req.first_time {
        let expected_identity = req
            .expected_identity
            .as_ref()
            .ok_or_else(|| anyhow!("normal flash requires expectedIdentity"))?;
        let identity = require_project_firmware_for_upgrade(state, device_id).await?;
        validate_device_identity(&identity, expected_identity)?;
    }

    {
        let mut inner = state.inner.lock().await;
        if inner.exclusive_ports.contains_key(&port_path) {
            return Err(anyhow!("device busy"));
        }
        inner
            .exclusive_ports
            .insert(port_path.clone(), "firmware flash".to_string());
    }
    let guard = ExclusiveGuard {
        state: state.clone(),
        port_path: port_path.clone(),
    };

    let file_path = resolve_catalog_file_path(&req.catalog_path, &app_file.path);
    let output = if req.first_time && app_file.kind == "elf" {
        Command::new("espflash")
            .env("ESPFLASH_SKIP_UPDATE_CHECK", "true")
            .arg("flash")
            .arg("--chip")
            .arg("esp32s3")
            .arg("--port")
            .arg(&port_path)
            .arg(&file_path)
            .output()
            .context("start espflash flash")?
    } else {
        let address = app_file
            .flash_address
            .unwrap_or(if app_file.kind == "full_image" {
                0
            } else {
                DEFAULT_FLASH_ADDRESS
            });
        Command::new("espflash")
            .env("ESPFLASH_SKIP_UPDATE_CHECK", "true")
            .arg("write-bin")
            .arg("--chip")
            .arg("esp32s3")
            .arg("--port")
            .arg(&port_path)
            .arg(format!("0x{address:x}"))
            .arg(&file_path)
            .output()
            .context("start espflash write-bin")?
    };
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        drop(guard);
        return Err(anyhow!("espflash failed: {log}"));
    }
    let captured_identity = if req.first_time {
        Some(capture_first_time_identity_after_flash(state, device_id).await?)
    } else {
        None
    };
    drop(guard);
    Ok(json!({
        "ok": true,
        "exit_code": output.status.code(),
        "artifact_id": artifact.artifact_id,
        "identity": captured_identity,
        "log": log,
    }))
}

async fn run_uploaded_flash_request(
    state: &AppState,
    device_id: &str,
    req: FirmwareUploadFlashRequest,
) -> anyhow::Result<Value> {
    if req.address != DEFAULT_FLASH_ADDRESS {
        return Err(anyhow!(
            "Local USB firmware flashing writes the app image at 0x10000"
        ));
    }
    let port_path = {
        let inner = state.inner.lock().await;
        inner
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow!("device not found"))?
            .usb
            .as_ref()
            .ok_or_else(|| anyhow!("device has no Local USB target"))?
            .port_path
            .clone()
    };
    ensure_port_allowed(&port_path)?;
    let identity = require_project_firmware_for_upgrade(state, device_id).await?;
    validate_device_identity(&identity, &req.expected_identity)?;

    let bytes = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(req.file_base64.trim())
            .context("firmware payload was not valid base64")?
    };
    let file_name = FsPath::new(req.file_name.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("firmware.bin");
    let temp_path = std::env::temp_dir().join(format!("isohub-flash-{}-{file_name}", next_id()));
    fs::write(&temp_path, bytes).with_context(|| format!("write {}", temp_path.display()))?;
    struct TempFirmwareFile(PathBuf);
    impl Drop for TempFirmwareFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _temp_file = TempFirmwareFile(temp_path.clone());

    {
        let mut inner = state.inner.lock().await;
        if inner.exclusive_ports.contains_key(&port_path) {
            return Err(anyhow!("device busy"));
        }
        inner
            .exclusive_ports
            .insert(port_path.clone(), "firmware flash".to_string());
    }
    let guard = ExclusiveGuard {
        state: state.clone(),
        port_path: port_path.clone(),
    };
    let output = Command::new("espflash")
        .env("ESPFLASH_SKIP_UPDATE_CHECK", "true")
        .arg("write-bin")
        .arg("--chip")
        .arg("esp32s3")
        .arg("--port")
        .arg(&port_path)
        .arg(format!("0x{:x}", req.address))
        .arg(&temp_path)
        .output()
        .context("start espflash write-bin")?;
    drop(guard);
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        return Err(anyhow!("espflash failed: {log}"));
    }
    Ok(json!({
        "ok": true,
        "exit_code": output.status.code(),
        "log": log,
    }))
}

#[derive(Clone)]
struct ExclusiveGuard {
    state: AppState,
    port_path: String,
}

impl Drop for ExclusiveGuard {
    fn drop(&mut self) {
        let state = self.state.clone();
        let port_path = self.port_path.clone();
        tokio::spawn(async move {
            state.inner.lock().await.exclusive_ports.remove(&port_path);
        });
    }
}

async fn require_lease(
    state: &AppState,
    device_id: &str,
    lease_id: Option<&str>,
) -> Result<(), Box<Response>> {
    cleanup_expired_leases(state).await;
    let Some(lease_id) = lease_id else {
        return Err(Box::new(unauthorized("lease_id is required")));
    };
    let inner = state.inner.lock().await;
    let Some(lease) = inner.leases.get(lease_id) else {
        return Err(Box::new(unauthorized("lease not found or expired")));
    };
    if lease.device_id != device_id {
        return Err(Box::new(unauthorized("lease does not belong to device")));
    }
    Ok(())
}

async fn cleanup_expired_leases(state: &AppState) {
    let now = Instant::now();
    state
        .inner
        .lock()
        .await
        .leases
        .retain(|_, lease| lease.expires_at > now);
}

fn list_serial_ports() -> anyhow::Result<Vec<UsbTarget>> {
    let targets = serialport::available_ports()?
        .into_iter()
        .filter_map(|port| {
            let (vendor_id, product_id, serial_number, manufacturer, product) = match port.port_type
            {
                serialport::SerialPortType::UsbPort(info) => (
                    Some(info.vid),
                    Some(info.pid),
                    info.serial_number,
                    info.manufacturer,
                    info.product,
                ),
                _ => (None, None, None, None, None),
            };
            let label = product
                .clone()
                .or(manufacturer)
                .unwrap_or_else(|| port.port_name.clone());
            let target = UsbTarget {
                port_path: port.port_name,
                label,
                vendor_id,
                product_id,
                serial_number,
            };
            is_esp32_serial_port(&target).then_some(target)
        })
        .filter(|target| ensure_port_allowed(&target.port_path).is_ok())
        .collect();
    Ok(dedupe_usb_serial_device_pairs(targets))
}

fn is_esp32_serial_port(port: &UsbTarget) -> bool {
    let path = port.port_path.to_lowercase();
    if path.contains("bluetooth") || path.contains("debug-console") {
        return false;
    }
    if port.vendor_id == Some(0x303a) {
        return true;
    }
    let label = port.label.to_lowercase();
    let path_looks_serial = path.contains("usbmodem")
        || path.contains("usbserial")
        || path.contains("ttyacm")
        || (path.starts_with("com") && path[3..].chars().all(|c| c.is_ascii_digit()));
    path_looks_serial
        && (label.contains("esp32") || label.contains("espressif") || label.contains("usb jtag"))
}

fn dedupe_usb_serial_device_pairs(mut targets: Vec<UsbTarget>) -> Vec<UsbTarget> {
    targets.sort_by(|a, b| {
        let a_cu = if is_cu_port(&a.port_path) { 0 } else { 1 };
        let b_cu = if is_cu_port(&b.port_path) { 0 } else { 1 };
        a_cu.cmp(&b_cu).then_with(|| a.port_path.cmp(&b.port_path))
    });

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for target in targets {
        let key = target
            .serial_number
            .clone()
            .unwrap_or_else(|| paired_serial_device_key(&target.port_path));
        if seen.insert(key) {
            deduped.push(target);
        }
    }
    deduped
}

fn is_cu_port(path: &str) -> bool {
    path.starts_with("/dev/cu.")
}

fn paired_serial_device_key(path: &str) -> String {
    path.replacen("/dev/tty.", "/dev/cu.", 1)
}

fn stable_usb_device_id(port_path: &str) -> String {
    let sanitized = port_path
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    format!("usb-{sanitized}")
}

fn stable_http_device_id(base_url: &str) -> String {
    let sanitized = base_url
        .trim()
        .trim_end_matches('/')
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    format!("http-{sanitized}")
}
