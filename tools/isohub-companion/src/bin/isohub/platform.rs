#[derive(Debug, Clone)]
struct ResolvedUsb {
    device: String,
    devd: String,
    identity: Option<DeviceIdentity>,
}

enum ResolvedTarget {
    Usb(ResolvedUsb),
    Http(String),
}

fn resolve_api_selector(
    selector: ApiSelectorArgs,
    default_devd: &str,
) -> anyhow::Result<ResolvedTarget> {
    let count = selector.selection_count();
    if count != 1 {
        return Err(anyhow!(
            "select exactly one of --hardware, --device, or --url"
        ));
    }
    if let Some(url) = selector.url {
        return Ok(ResolvedTarget::Http(url));
    }
    if let Some(device) = selector.device {
        return Ok(ResolvedTarget::Usb(ResolvedUsb {
            device,
            devd: default_devd.to_string(),
            identity: None,
        }));
    }
    let hardware_id = selector.hardware.expect("count checked");
    let hardware = find_hardware(&hardware_id)?;
    let identity = hardware.identity.clone();
    match hardware.transport {
        HardwareTransport::Usb {
            device_id,
            devd_url,
        } => {
            let _ = devd_url;
            Ok(ResolvedTarget::Usb(ResolvedUsb {
                device: device_id,
                devd: default_devd.to_string(),
                identity,
            }))
        }
        HardwareTransport::Http { base_url } => Ok(ResolvedTarget::Http(base_url)),
        HardwareTransport::WebSerial { .. } => Err(anyhow!(
            "saved hardware {hardware_id} uses Web Serial; CLI automation requires devd USB or HTTP"
        )),
    }
}

fn resolve_usb_device(
    selector: &UsbSelectorArgs,
    default_devd: &str,
) -> anyhow::Result<ResolvedUsb> {
    if selector.hardware.is_some() == selector.device.is_some() {
        return Err(anyhow!("select exactly one of --hardware or --device"));
    }
    if let Some(device) = selector.device.clone() {
        return Ok(ResolvedUsb {
            device,
            devd: default_devd.to_string(),
            identity: None,
        });
    }
    let hardware_id = selector.hardware.as_ref().expect("checked");
    let hardware = find_hardware(hardware_id)?;
    let identity = hardware.identity.clone();
    match hardware.transport {
        HardwareTransport::Usb {
            device_id,
            devd_url,
        } => {
            let _ = devd_url;
            Ok(ResolvedUsb {
                device: device_id,
                devd: default_devd.to_string(),
                identity,
            })
        }
        _ => Err(anyhow!(
            "saved hardware {hardware_id} is not devd USB hardware"
        )),
    }
}

fn resolve_usb_capable_selector(
    selector: ApiSelectorArgs,
    default_devd: &str,
    capability: &str,
) -> anyhow::Result<ResolvedUsb> {
    let count = selector.selection_count();
    if count != 1 {
        return Err(anyhow!(
            "select exactly one of --hardware, --device, or --url"
        ));
    }
    if selector.url.is_some() {
        return Err(anyhow!(
            "{capability} require Local USB in the CLI; --url is read-only"
        ));
    }
    if let Some(device) = selector.device {
        return Ok(ResolvedUsb {
            device,
            devd: default_devd.to_string(),
            identity: None,
        });
    }
    let hardware_id = selector.hardware.expect("count checked");
    let hardware = find_hardware(&hardware_id)?;
    let identity = hardware.identity.clone();
    match hardware.transport {
        HardwareTransport::Usb {
            device_id,
            devd_url,
        } => {
            let _ = devd_url;
            Ok(ResolvedUsb {
                device: device_id,
                devd: default_devd.to_string(),
                identity,
            })
        }
        HardwareTransport::Http { .. } => Err(anyhow!(
            "{capability} require Local USB in the CLI; saved hardware {hardware_id} is Wi-Fi/LAN only"
        )),
        HardwareTransport::WebSerial { .. } => Err(anyhow!(
            "{capability} require Local USB in the CLI; saved hardware {hardware_id} uses Web Serial and is not available to CLI automation"
        )),
    }
}

fn find_hardware(id: &str) -> anyhow::Result<DeviceProfile> {
    read_hardware_registry()?
        .devices
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| anyhow!("saved hardware not found: {id}"))
}

async fn devd_request(
    _client: &Client,
    devd: &DevdClient,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let (ipc_method, params) = map_devd_ipc_endpoint(method, path, body)?;
    devd_ipc_call(devd, &ipc_method, params).await
}

async fn devd_ipc_call(devd: &DevdClient, method: &str, params: Value) -> anyhow::Result<Value> {
    match ipc_call(&devd.endpoint, method, params.clone()).await {
        Ok(value) => Ok(value),
        Err(err) if devd.auto_start && looks_like_transient_ipc_error(&err) => {
            let start_mode = acquire_devd_start_gate(&devd.endpoint)?;
            if matches!(start_mode, DevdStartMode::Spawned { .. }) {
                start_devd(&devd.endpoint)?;
            }
            match wait_for_devd(&devd.endpoint, method, params.clone()).await {
                Ok(value) => Ok(value),
                Err(wait_err) if matches!(start_mode, DevdStartMode::WaitingForExisting) => {
                    clear_devd_start_gate(&devd.endpoint)?;
                    let retry_mode = acquire_devd_start_gate(&devd.endpoint)?;
                    if matches!(retry_mode, DevdStartMode::Spawned { .. }) {
                        start_devd(&devd.endpoint)?;
                    }
                    wait_for_devd(&devd.endpoint, method, params).await.map_err(|_| wait_err)
                }
                Err(wait_err) => Err(wait_err),
            }
        }
        Err(err) => Err(err),
    }
}

fn looks_like_transient_ipc_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let message = cause.to_string();
        message.contains("connect IPC")
            || message.contains("Connection refused")
            || message.contains("IPC daemon closed the connection without a response")
    })
}

enum DevdStartMode {
    Spawned {
        _gate: DevdStartGate,
    },
    WaitingForExisting,
}

struct DevdStartGate {
    path: PathBuf,
}

impl Drop for DevdStartGate {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_devd_start_gate(endpoint: &str) -> anyhow::Result<DevdStartMode> {
    let path = devd_start_gate_path(endpoint);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create devd start gate dir {}", parent.display()))?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write as _;

            let _ = writeln!(file, "pid={}", std::process::id());
            let _ = writeln!(file, "endpoint={endpoint}");
            Ok(DevdStartMode::Spawned {
                _gate: DevdStartGate { path },
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(DevdStartMode::WaitingForExisting)
        }
        Err(err) => Err(err).with_context(|| format!("create devd start gate {}", path.display())),
    }
}

fn clear_devd_start_gate(endpoint: &str) -> anyhow::Result<()> {
    let path = devd_start_gate_path(endpoint);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove devd start gate {}", path.display())),
    }
}

fn devd_start_gate_path(endpoint: &str) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&endpoint, &mut hasher);
    let hash = std::hash::Hasher::finish(&hasher);
    std::env::temp_dir()
        .join("isohub")
        .join(format!("devd-start-{hash:016x}.lock"))
}

fn start_devd(endpoint: &str) -> anyhow::Result<()> {
    let devd_bin = std::env::var_os("ISOHUB_DEVD_BIN")
        .map(PathBuf::from)
        .or_else(|| {
            let mut path = std::env::current_exe().ok()?;
            let suffix = std::env::consts::EXE_SUFFIX;
            path.set_file_name(format!("isohub-devd{suffix}"));
            Some(path)
        })
        .ok_or_else(|| anyhow!("cannot resolve isohub-devd path"))?;
    if !devd_bin.is_file() {
        return Err(anyhow!(
            "isohub-devd was not found next to isohub; run `cargo build --manifest-path tools/isohub-companion/Cargo.toml --bin isohub-devd` or set ISOHUB_DEVD_BIN"
        ));
    }
    ProcessCommand::new(devd_bin)
        .arg("serve")
        .arg("--endpoint")
        .arg(endpoint)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("start isohub-devd IPC daemon")?;
    Ok(())
}

async fn wait_for_devd(endpoint: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let deadline = Instant::now() + devd_start_timeout();
    let mut last_error = None;
    while Instant::now() < deadline {
        match ipc_call(endpoint, method, params.clone()).await {
            Ok(value) => return Ok(value),
            Err(err) => {
                last_error = Some(err);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("isohub-devd IPC daemon did not start")))
}

fn devd_start_timeout() -> Duration {
    if let Ok(value) = std::env::var("ISOHUB_DEVD_START_TIMEOUT_SECS")
        && let Ok(seconds) = value.trim().parse::<u64>()
        && seconds > 0
    {
        return Duration::from_secs(seconds);
    }
    if std::env::var_os("ISOHUB_DEVD_BIN").is_some() {
        return Duration::from_secs(60);
    }
    Duration::from_secs(4)
}

fn map_devd_ipc_endpoint(
    method: Method,
    path: &str,
    body: Option<Value>,
) -> anyhow::Result<(String, Value)> {
    let (path_only, query) = path.split_once('?').unwrap_or((path, ""));
    if method == Method::GET && path_only == "/api/v1/devices" {
        return Ok(("devices.list".to_string(), json!({})));
    }
    if method == Method::POST && path_only == "/api/v1/devices/scan" {
        return Ok(("devices.scan".to_string(), json!({})));
    }
    if method == Method::POST && path_only == "/api/v1/serial/lease" {
        return Ok((
            "serial.lease.create".to_string(),
            body.unwrap_or_else(|| json!({})),
        ));
    }
    if method == Method::DELETE
        && let Some(lease_id) = path_only.strip_prefix("/api/v1/serial/lease/")
    {
        return Ok((
            "serial.lease.release".to_string(),
            json!({"lease_id": lease_id}),
        ));
    }
    let Some(rest) = path_only.strip_prefix("/api/v1/devices/") else {
        return Err(anyhow!("unsupported devd IPC endpoint: {method} {path}"));
    };
    let (device_id, suffix) = rest
        .split_once('/')
        .ok_or_else(|| anyhow!("invalid devd device path: {path}"))?;
    let mut params = json!({"device_id": device_id});
    let params_map = params.as_object_mut().expect("object");

    let ipc_method = match (method.as_str(), suffix) {
        ("GET", "status") => "device.status",
        ("GET", "wifi") => "device.wifi.get",
        ("POST", "wifi") => {
            merge_body(params_map, body);
            "device.wifi.set"
        }
        ("DELETE", "wifi") => "device.wifi.clear",
        ("GET", "ports") => "device.ports.get",
        ("GET", "diag-snapshot") => "device.hardware.snapshot",
        ("GET", "session") => {
            if let Some(tail) = query
                .split('&')
                .find_map(|part| part.strip_prefix("tail="))
                .and_then(|tail| tail.parse::<usize>().ok())
            {
                params_map.insert("tail".to_string(), json!(tail));
            }
            "device.session"
        }
        ("POST", "flash") => {
            merge_body(params_map, body);
            "device.flash"
        }
        ("POST", "reset") => {
            merge_body(params_map, body);
            "device.reset"
        }
        ("GET", "diagnostics") => "device.diagnostics",
        ("POST", _) if suffix.starts_with("ports/") && suffix.ends_with("/replug") => {
            let port = suffix
                .trim_start_matches("ports/")
                .trim_end_matches("/replug");
            params_map.insert("port".to_string(), json!(port));
            "device.port.replug"
        }
        ("POST", _) if suffix.starts_with("ports/") && suffix.contains("/power") => {
            let port = suffix
                .trim_start_matches("ports/")
                .trim_end_matches("/power");
            let enabled = query
                .split('&')
                .find_map(|part| part.strip_prefix("enabled="))
                .ok_or_else(|| anyhow!("enabled query is required"))?
                .parse::<bool>()
                .context("enabled must be a boolean")?;
            params_map.insert("port".to_string(), json!(port));
            params_map.insert("enabled".to_string(), json!(enabled));
            "device.port.power"
        }
        _ => return Err(anyhow!("unsupported devd IPC endpoint: {method} {path}")),
    };
    Ok((ipc_method.to_string(), params))
}

fn merge_body(target: &mut serde_json::Map<String, Value>, body: Option<Value>) {
    if let Some(Value::Object(map)) = body {
        target.extend(map);
    }
}

async fn request_selected(
    client: &Client,
    devd: &DevdClient,
    selector: ApiSelectorArgs,
    method: Method,
    suffix: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let selected = resolve_api_selector(selector, &devd.endpoint)?;
    match selected {
        ResolvedTarget::Usb(usb) => {
            let usb_devd = devd.with_endpoint(usb.devd.clone());
            ensure_devd_device_registered(client, &usb_devd, &usb.device).await?;
            devd_request(
                client,
                &usb_devd,
                method,
                &format!("/api/v1/devices/{}{}", usb.device, suffix),
                body,
            )
            .await
        }
        ResolvedTarget::Http(url) => {
            let (http_method, path, http_body) = map_http_endpoint(method, suffix, body)?;
            let mut request = client.request(http_method, api_url(&url, &path)?);
            if let Some(body) = http_body {
                request = request.json(&body);
            }
            Ok(request
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await?)
        }
    }
}

async fn request_selected_usb_capable(
    client: &Client,
    devd: &DevdClient,
    selector: ApiSelectorArgs,
    method: Method,
    suffix: &str,
    body: Option<Value>,
    capability: &str,
) -> anyhow::Result<Value> {
    let usb = resolve_usb_capable_selector(selector, &devd.endpoint, capability)?;
    let usb_devd = devd.with_endpoint(usb.devd.clone());
    ensure_devd_device_registered(client, &usb_devd, &usb.device).await?;
    devd_request(
        client,
        &usb_devd,
        method,
        &format!("/api/v1/devices/{}{}", usb.device, suffix),
        body,
    )
    .await
}

fn map_http_endpoint(
    method: Method,
    suffix: &str,
    body: Option<Value>,
) -> anyhow::Result<(Method, String, Option<Value>)> {
    let mapped = match (method.as_str(), suffix) {
        ("GET", "/status") => (method, "/api/v1/info".to_string(), body),
        ("GET", "/wifi") => (method, "/api/v1/wifi".to_string(), body),
        ("POST", "/wifi") => (Method::POST, "/api/v1/wifi/set".to_string(), body),
        ("DELETE", "/wifi") => (Method::POST, "/api/v1/wifi/clear".to_string(), body),
        ("GET", "/ports") => (method, "/api/v1/ports".to_string(), body),
        ("GET", "/diag-snapshot") => (method, "/api/v1/diag-snapshot".to_string(), body),
        ("GET", "/diagnostics") => (method, "/api/v1/pd-diagnostics".to_string(), body),
        ("POST", _) if suffix.starts_with("/ports/") && suffix.ends_with("/replug") => {
            let port = suffix
                .trim_start_matches("/ports/")
                .trim_end_matches("/replug");
            (
                Method::POST,
                format!("/api/v1/ports/{port}/actions/replug"),
                None,
            )
        }
        ("POST", _) if suffix.starts_with("/ports/") && suffix.contains("/power?enabled=") => {
            let rest = suffix.trim_start_matches("/ports/");
            let (port, query) = rest
                .split_once("/power?")
                .ok_or_else(|| anyhow!("invalid port power path"))?;
            (
                Method::POST,
                format!("/api/v1/ports/{port}/power?{query}"),
                None,
            )
        }
        _ => (method, suffix.to_string(), body),
    };
    Ok(mapped)
}

async fn handle_ports(
    client: &Client,
    devd: &DevdClient,
    selector: ApiSelectorArgs,
    command: Option<PortsCommand>,
) -> anyhow::Result<Value> {
    match command {
        None => request_selected(client, devd, selector, Method::GET, "/ports", None).await,
        Some(PortsCommand::Power { port, enabled }) => {
            request_selected(
                client,
                devd,
                selector,
                Method::POST,
                &format!("/ports/{}/power?enabled={enabled}", port.as_str()),
                None,
            )
            .await
        }
        Some(PortsCommand::Replug { port }) => {
            request_selected(
                client,
                devd,
                selector,
                Method::POST,
                &format!("/ports/{}/replug", port.as_str()),
                None,
            )
            .await
        }
    }
}

async fn handle_hardware(
    client: &Client,
    devd: &DevdClient,
    command: HardwareCommand,
) -> anyhow::Result<Value> {
    let path = registry_path()?;
    match command {
        HardwareCommand::Path => Ok(json!({"path": path})),
        HardwareCommand::List | HardwareCommand::Recent => {
            let mut registry = read_hardware_registry()?;
            registry
                .devices
                .sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
            Ok(json!({"path": path, "devices": registry.devices}))
        }
        HardwareCommand::Available { scan } => {
            let registry = read_hardware_registry()?;
            let devd_devices = if scan {
                devd_request(client, devd, Method::POST, "/api/v1/devices/scan", None).await
            } else {
                devd_request(client, devd, Method::GET, "/api/v1/devices", None).await
            };
            Ok(json!({
                "path": path,
                "saved": registry.devices,
                "devd": devd_devices.unwrap_or_else(|err| json!({"error": err.to_string()})),
            }))
        }
        HardwareCommand::Save {
            id,
            name,
            transport,
            device,
            url,
        } => {
            let transport = match transport {
                TransportArg::Usb => HardwareTransport::Usb {
                    device_id: device
                        .ok_or_else(|| anyhow!("--device is required for usb hardware"))?,
                    devd_url: None,
                },
                TransportArg::Http => HardwareTransport::Http {
                    base_url: url.ok_or_else(|| anyhow!("--url is required for http hardware"))?,
                },
                TransportArg::WebSerial => HardwareTransport::WebSerial { label: device },
            };
            let saved = save_hardware(SavedHardwareInput {
                id,
                name,
                transport,
                identity: None,
            })?;
            Ok(json!({"path": path, "device": saved}))
        }
        HardwareCommand::Forget { id } => {
            let mut registry = read_hardware_registry()?;
            let before = registry.devices.len();
            registry.devices.retain(|device| device.id != id);
            isohub_companion::write_hardware_registry(&registry)?;
            Ok(json!({"path": path, "id": id, "removed": before != registry.devices.len()}))
        }
    }
}

async fn handle_discover(client: &Client, devd: &DevdClient, scan: bool) -> anyhow::Result<Value> {
    let registry = read_hardware_registry()?;
    let usb_devices = discover_usb_devices(client, devd, scan, &registry.devices).await?;
    let (lan_devices, warnings) = discover_lan_devices(client, &registry.devices).await;

    let mut devices = Vec::with_capacity(lan_devices.len() + usb_devices.len());
    devices.extend(lan_devices);
    devices.extend(usb_devices);

    if warnings.is_empty() {
        Ok(json!({ "devices": devices }))
    } else {
        Ok(json!({
            "devices": devices,
            "warnings": warnings,
        }))
    }
}

async fn handle_flash(
    client: &Client,
    devd: &DevdClient,
    args: FlashArgs,
) -> anyhow::Result<Value> {
    let device = resolve_usb_device(&args.selector, &devd.endpoint)?;
    let device_devd = devd.with_endpoint(device.devd.clone());
    let expected_identity = DeviceIdentity {
        device_id: args.expected_device_id.clone().or_else(|| {
            device
                .identity
                .as_ref()
                .and_then(|identity| identity.device_id.clone())
        }),
        mac: args.expected_mac.clone().or_else(|| {
            device
                .identity
                .as_ref()
                .and_then(|identity| identity.mac.clone())
        }),
    };
    if args.real
        && !args.first_time
        && expected_identity.device_id.is_none()
        && expected_identity.mac.is_none()
    {
        return Err(anyhow!(
            "normal flash requires --expected-device-id/--expected-mac or saved hardware identity"
        ));
    }
    let catalog: FirmwareCatalog =
        serde_json::from_slice(&fs::read(&args.catalog).context("read firmware catalog")?)?;
    let artifact = catalog
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == args.artifact)
        .ok_or_else(|| anyhow!("artifact not found in catalog: {}", args.artifact))?;

    let mut confirm_non_project_firmware = args.confirm_non_project_firmware;
    if args.first_time && args.real && !confirm_non_project_firmware {
        if !std::io::stdin().is_terminal() {
            return Err(anyhow!(
                "first-time flash may target download-mode or non-project firmware; rerun interactively or pass --confirm-non-project-firmware after external target confirmation"
            ));
        }
        eprintln!("First-time full flash requested.");
        eprintln!("device={}", device.device);
        eprintln!("artifact={}", artifact.artifact_id);
        eprintln!("target={}", artifact.target);
        eprintln!("Type 'flash {}' to continue:", artifact.artifact_id);
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if line.trim() != format!("flash {}", artifact.artifact_id) {
            return Err(anyhow!("first-time flash confirmation did not match"));
        }
        confirm_non_project_firmware = true;
    }

    devd_device_post_with_lease(
        client,
        &device_devd,
        &device.device,
        "/flash",
        json!({
            "catalog_path": args.catalog,
            "artifact_id": args.artifact,
            "real": args.real,
            "first_time": args.first_time,
            "confirm_non_project_firmware": confirm_non_project_firmware,
            "expected_identity": expected_identity,
        }),
    )
    .await
}

async fn devd_device_post_with_lease(
    client: &Client,
    devd: &DevdClient,
    device: &str,
    suffix: &str,
    mut body: Value,
) -> anyhow::Result<Value> {
    ensure_devd_device_registered(client, devd, device).await?;
    let lease = create_lease(client, devd, device).await?;
    if let Some(map) = body.as_object_mut() {
        map.insert(
            "lease_id".to_string(),
            Value::String(lease.lease_id.clone()),
        );
    }
    let result = devd_request(
        client,
        devd,
        Method::POST,
        &format!("/api/v1/devices/{device}{suffix}"),
        Some(body),
    )
    .await;
    let _ = devd_request(
        client,
        devd,
        Method::DELETE,
        &format!("/api/v1/serial/lease/{}", lease.lease_id),
        None,
    )
    .await;
    result
}

async fn ensure_devd_device_registered(
    client: &Client,
    devd: &DevdClient,
    device: &str,
) -> anyhow::Result<()> {
    let value = devd_request(client, devd, Method::POST, "/api/v1/devices/scan", None).await?;
    let found = value
        .get("devices")
        .and_then(Value::as_array)
        .is_some_and(|devices| {
            devices
                .iter()
                .any(|entry| entry.get("id").and_then(Value::as_str) == Some(device))
        });
    if !found {
        return Err(anyhow!("device not found after scan: {device}"));
    }
    Ok(())
}

async fn create_lease(
    client: &Client,
    devd: &DevdClient,
    device: &str,
) -> anyhow::Result<CliLease> {
    let value = devd_request(
        client,
        devd,
        Method::POST,
        "/api/v1/serial/lease",
        Some(json!({"device_id": device})),
    )
    .await?;
    Ok(serde_json::from_value(value)?)
}

fn ensure_success_envelope(value: &Value) -> anyhow::Result<()> {
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let message = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .or_else(|| value.get("error").and_then(Value::as_str))
            .unwrap_or("device returned ok=false");
        return Err(anyhow!("device request failed: {message}"));
    }
    Ok(())
}
