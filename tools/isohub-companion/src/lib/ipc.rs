pub async fn serve_ipc(config: IpcConfig) -> anyhow::Result<()> {
    let state = AppState::new("ipc://isohub-devd");
    serve_ipc_with_state(config, state).await
}

async fn serve_ipc_with_state(config: IpcConfig, state: AppState) -> anyhow::Result<()> {
    let runtime = IpcRuntime::new(state);
    #[cfg(unix)]
    {
        serve_ipc_unix(config, runtime).await
    }
    #[cfg(windows)]
    {
        serve_ipc_windows(config, runtime).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (config, runtime);
        Err(anyhow!("isohub-devd IPC is unsupported on this platform"))
    }
}

#[cfg(unix)]
async fn serve_ipc_unix(config: IpcConfig, runtime: IpcRuntime) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    use tokio::net::UnixListener;

    let path = PathBuf::from(&config.endpoint);
    if let Some(parent) = path.parent() {
        let created_parent = !parent.exists();
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        if created_parent {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("chmod {}", parent.display()))?;
        }
    }
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("remove stale {}", path.display()))?;
    }
    let listener =
        UnixListener::bind(&path).with_context(|| format!("bind IPC {}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("chmod {}", path.display()))?;
    tracing::info!("isohub-devd IPC listening on {}", path.display());
    let cleanup_path = path.clone();
    loop {
        if let Some(idle_timeout) = config.idle_timeout {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _) = accepted?;
                    spawn_ipc_client(stream, runtime.clone()).await;
                }
                _ = tokio::time::sleep(idle_timeout) => {
                    if ipc_should_shutdown(&runtime, idle_timeout).await {
                        tracing::info!("isohub-devd IPC idle timeout reached; shutting down");
                        break;
                    }
                }
            }
        } else {
            let (stream, _) = listener.accept().await?;
            spawn_ipc_client(stream, runtime.clone()).await;
        }
    }
    let _ = fs::remove_file(cleanup_path);
    Ok(())
}

#[cfg(windows)]
async fn serve_ipc_windows(config: IpcConfig, runtime: IpcRuntime) -> anyhow::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    tracing::info!("isohub-devd IPC listening on {}", config.endpoint);
    loop {
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(&config.endpoint)
            .with_context(|| format!("create IPC pipe {}", config.endpoint))?;
        if let Some(idle_timeout) = config.idle_timeout {
            tokio::select! {
                connected = server.connect() => {
                    connected.context("connect IPC pipe client")?;
                    spawn_ipc_client(server, runtime.clone()).await;
                }
                _ = tokio::time::sleep(idle_timeout) => {
                    if ipc_should_shutdown(&runtime, idle_timeout).await {
                        tracing::info!("isohub-devd IPC idle timeout reached; shutting down");
                        break;
                    }
                }
            }
        } else {
            server.connect().await.context("connect IPC pipe client")?;
            spawn_ipc_client(server, runtime.clone()).await;
        }
    }
    Ok(())
}

async fn spawn_ipc_client<S>(stream: S, runtime: IpcRuntime)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    ipc_client_connected(&runtime).await;
    tokio::spawn(async move {
        if let Err(err) = handle_ipc_stream(stream, runtime.clone()).await {
            tracing::warn!("IPC client failed: {err:#}");
        }
        ipc_client_disconnected(&runtime).await;
    });
}

async fn ipc_client_connected(runtime: &IpcRuntime) {
    let mut lifecycle = runtime.lifecycle.lock().await;
    lifecycle.active_clients += 1;
    lifecycle.last_activity = Instant::now();
}

async fn ipc_client_disconnected(runtime: &IpcRuntime) {
    let mut lifecycle = runtime.lifecycle.lock().await;
    lifecycle.active_clients = lifecycle.active_clients.saturating_sub(1);
    lifecycle.last_activity = Instant::now();
}

async fn ipc_mark_activity(runtime: &IpcRuntime) {
    runtime.lifecycle.lock().await.last_activity = Instant::now();
}

async fn ipc_should_shutdown(runtime: &IpcRuntime, idle_timeout: Duration) -> bool {
    let lifecycle = runtime.lifecycle.lock().await;
    lifecycle.active_clients == 0 && lifecycle.last_activity.elapsed() >= idle_timeout
}

async fn handle_ipc_stream<S>(stream: S, runtime: IpcRuntime) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<IpcRequest>(&line) {
            Ok(request) => handle_ipc_request(&runtime.app, request).await,
            Err(err) => IpcResponse {
                id: "invalid".to_string(),
                ok: false,
                result: None,
                error: Some(format!("invalid IPC request: {err}")),
            },
        };
        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        write.write_all(&encoded).await?;
        write.flush().await?;
        ipc_mark_activity(&runtime).await;
    }
    Ok(())
}

async fn handle_ipc_request(state: &AppState, request: IpcRequest) -> IpcResponse {
    let id = request.id;
    let result = dispatch_ipc_request(state, &request.method, request.params).await;
    match result {
        Ok(result) => IpcResponse {
            id,
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(err) => IpcResponse {
            id,
            ok: false,
            result: None,
            error: Some(err.to_string()),
        },
    }
}

async fn dispatch_ipc_request(
    state: &AppState,
    method: &str,
    params: Value,
) -> anyhow::Result<Value> {
    match method {
        "devd.health" => Ok(json!({"ok": true})),
        "devices.list" => ipc_list_devices(state).await,
        "devices.scan" => ipc_scan_devices(state).await,
        "device.status" => {
            let req: DeviceIdRequest = serde_json::from_value(params)?;
            let value = require_compatible_project_firmware(state, &req.device_id).await?;
            if let Err(err) = cache_project_firmware_info(state, &req.device_id, &value).await {
                tracing::debug!(
                    device_id = %req.device_id,
                    "could not cache project firmware info: {err}"
                );
            }
            Ok(redact_sensitive(&value))
        }
        "device.session" => {
            let req: DeviceSessionRequest = serde_json::from_value(params)?;
            ipc_device_session(state, &req.device_id, req.tail, req.lease_id).await
        }
        "device.wifi.get" => {
            let req: DeviceIdRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware(state, &req.device_id).await?;
            let value = usb_jsonl_request(state, &req.device_id, "wifi.get", None).await?;
            if let Err(err) = update_http_profile_from_usb_wifi(state, &req.device_id, &value).await
            {
                tracing::warn!(
                    device_id = %req.device_id,
                    "could not update HTTP profile from Wi-Fi status: {err}"
                );
            }
            Ok(redact_sensitive(&value))
        }
        "device.wifi.set" => {
            let req: DeviceWifiSetRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware(state, &req.device_id).await?;
            let expected_ssid = req.ssid.clone();
            match usb_jsonl_request(
                state,
                &req.device_id,
                "wifi.set",
                Some(json!({"ssid": req.ssid, "psk": req.psk})),
            )
            .await
            {
                Ok(_) => {
                    let value =
                        verify_wifi_after_set_timeout(state, &req.device_id, &expected_ssid)
                            .await?;
                    if let Err(err) =
                        update_http_profile_from_usb_wifi(state, &req.device_id, &value).await
                    {
                        tracing::warn!(
                            device_id = %req.device_id,
                            "could not update HTTP profile from Wi-Fi status: {err}"
                        );
                    }
                    Ok(redact_sensitive(&value))
                }
                Err(err) => {
                    match verify_wifi_after_set_timeout(state, &req.device_id, &expected_ssid).await
                    {
                        Ok(value) => {
                            if let Err(update_err) =
                                update_http_profile_from_usb_wifi(state, &req.device_id, &value)
                                    .await
                            {
                                tracing::warn!(
                                    device_id = %req.device_id,
                                    "could not update HTTP profile from Wi-Fi status: {update_err}"
                                );
                            }
                            Ok(redact_sensitive(&value))
                        }
                        Err(_) => Err(err),
                    }
                }
            }
        }
        "device.wifi.clear" => {
            let req: DeviceIdRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware(state, &req.device_id).await?;
            usb_jsonl_request(state, &req.device_id, "wifi.clear", None).await?;
            let value = verify_wifi_after_clear_timeout(state, &req.device_id).await?;
            if let Err(err) = delete_http_profile_for_usb_device(state, &req.device_id).await {
                tracing::warn!(
                    device_id = %req.device_id,
                    "could not delete HTTP profile after Wi-Fi clear: {err}"
                );
            }
            Ok(redact_sensitive(&value))
        }
        "device.ports.get" => {
            let req: DeviceIdRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware_fast(state, &req.device_id).await?;
            Ok(redact_sensitive(
                &usb_jsonl_request(state, &req.device_id, "ports.get", None).await?,
            ))
        }
        "device.port.power" => {
            let req: DevicePortPowerRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware_fast(state, &req.device_id).await?;
            Ok(redact_sensitive(
                &usb_jsonl_request(
                    state,
                    &req.device_id,
                    "port.power_set",
                    Some(json!({"port": req.port, "enabled": req.enabled})),
                )
                .await?,
            ))
        }
        "device.port.replug" => {
            let req: DevicePortRequest = serde_json::from_value(params)?;
            require_compatible_project_firmware_fast(state, &req.device_id).await?;
            Ok(redact_sensitive(
                &usb_jsonl_request(
                    state,
                    &req.device_id,
                    "port.replug",
                    Some(json!({"port": req.port})),
                )
                .await?,
            ))
        }
        "serial.lease.create" => {
            let req: LeaseRequest = serde_json::from_value(params)?;
            ipc_create_lease(state, req).await
        }
        "serial.lease.release" => {
            let req: LeaseIdRequest = serde_json::from_value(params)?;
            ipc_release_lease(state, &req.lease_id).await
        }
        "device.flash" => {
            let req: DeviceFlashRequest = serde_json::from_value(params)?;
            require_lease_value(state, &req.device_id, req.flash.lease_id.as_deref()).await?;
            Ok(redact_sensitive(
                &run_flash_request(state, &req.device_id, req.flash).await?,
            ))
        }
        "device.reset" => {
            let req: DeviceResetRequest = serde_json::from_value(params)?;
            ipc_device_reset(state, &req.device_id, req.lease_id.as_deref()).await
        }
        "device.diagnostics" => {
            let req: DeviceIdRequest = serde_json::from_value(params)?;
            Ok(redact_sensitive(
                &build_device_diagnostics(state, &req.device_id).await?,
            ))
        }
        "firmware.catalog.validate" => {
            let catalog: FirmwareCatalog = serde_json::from_value(params)?;
            let errors = validate_catalog_shape(&catalog);
            Ok(json!({"ok": errors.is_empty(), "errors": errors}))
        }
        _ => Err(anyhow!("unknown IPC method: {method}")),
    }
}

#[derive(Debug, Deserialize)]
struct DeviceIdRequest {
    device_id: String,
}

#[derive(Debug, Deserialize)]
struct DeviceSessionRequest {
    device_id: String,
    lease_id: Option<String>,
    tail: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DeviceWifiSetRequest {
    device_id: String,
    ssid: String,
    psk: String,
}

#[derive(Debug, Deserialize)]
struct DevicePortRequest {
    device_id: String,
    port: String,
}

#[derive(Debug, Deserialize)]
struct DevicePortPowerRequest {
    device_id: String,
    port: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct LeaseIdRequest {
    lease_id: String,
}

#[derive(Debug, Deserialize)]
struct DeviceFlashRequest {
    device_id: String,
    #[serde(flatten)]
    flash: FlashRequest,
}

#[derive(Debug, Deserialize)]
struct DeviceResetRequest {
    device_id: String,
    lease_id: Option<String>,
}

async fn ipc_list_devices(state: &AppState) -> anyhow::Result<Value> {
    cleanup_expired_leases(state).await;
    let devices = state
        .inner
        .lock()
        .await
        .devices
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({"devices": devices}))
}

async fn ipc_scan_devices(state: &AppState) -> anyhow::Result<Value> {
    let ports = list_serial_ports().context("serial enumeration failed")?;
    let mut inner = state.inner.lock().await;
    reconcile_scanned_usb_devices(&mut inner, ports);
    let devices = inner.devices.values().cloned().collect::<Vec<_>>();
    Ok(json!({"devices": devices}))
}

async fn ipc_device_session(
    state: &AppState,
    device_id: &str,
    tail: Option<usize>,
    lease_id: Option<String>,
) -> anyhow::Result<Value> {
    let tail = tail.unwrap_or(200).min(MAX_SESSION_ITEMS);
    let inner = state.inner.lock().await;
    let device = inner
        .devices
        .get(device_id)
        .ok_or_else(|| anyhow!("device not found"))?;
    if let Some(lease_id) = lease_id.as_deref()
        && !inner.leases.contains_key(lease_id)
    {
        return Err(anyhow!("lease not found or expired"));
    }
    Ok(json!({
        "logs": tail_items(&device.session.logs, tail),
        "traces": tail_items(&device.session.traces, tail),
    }))
}

async fn ipc_create_lease(state: &AppState, req: LeaseRequest) -> anyhow::Result<Value> {
    cleanup_expired_leases(state).await;
    let port_path = {
        let inner = state.inner.lock().await;
        let device = inner
            .devices
            .get(&req.device_id)
            .ok_or_else(|| anyhow!("device not found"))?;
        device.usb.as_ref().map(|usb| usb.port_path.clone())
    };

    let lease_id = next_id();
    let lease = LeaseRecord {
        lease_id: lease_id.clone(),
        device_id: req.device_id.clone(),
        port_path,
        expires_at: Instant::now() + Duration::from_millis(LEASE_TTL_MS),
    };
    state
        .inner
        .lock()
        .await
        .leases
        .insert(lease_id.clone(), lease);
    Ok(json!(LeaseResponse {
        lease_id,
        device_id: req.device_id,
        heartbeat_interval_ms: LEASE_HEARTBEAT_INTERVAL_MS,
        lease_ttl_ms: LEASE_TTL_MS,
    }))
}

async fn ipc_release_lease(state: &AppState, lease_id: &str) -> anyhow::Result<Value> {
    let removed = state.inner.lock().await.leases.remove(lease_id).is_some();
    Ok(json!({"ok": true, "released": removed}))
}

async fn require_lease_value(
    state: &AppState,
    device_id: &str,
    lease_id: Option<&str>,
) -> anyhow::Result<()> {
    cleanup_expired_leases(state).await;
    let lease_id = lease_id.ok_or_else(|| anyhow!("lease_id is required"))?;
    let inner = state.inner.lock().await;
    let lease = inner
        .leases
        .get(lease_id)
        .ok_or_else(|| anyhow!("lease not found or expired"))?;
    if lease.device_id != device_id {
        return Err(anyhow!("lease does not belong to device"));
    }
    Ok(())
}

async fn ipc_device_reset(
    state: &AppState,
    device_id: &str,
    lease_id: Option<&str>,
) -> anyhow::Result<Value> {
    require_lease_value(state, device_id, lease_id).await?;
    require_compatible_project_firmware(state, device_id).await?;
    let port_path = device_usb_port_path(state, device_id).await?;
    {
        let mut inner = state.inner.lock().await;
        if inner.exclusive_ports.contains_key(&port_path) {
            return Err(anyhow!("device busy"));
        }
        inner
            .exclusive_ports
            .insert(port_path.clone(), "reset".to_string());
    }
    let guard = ExclusiveGuard {
        state: state.clone(),
        port_path,
    };
    let result =
        usb_jsonl_request_with_exclusive(state, device_id, "reboot", None, Some("reset")).await;
    drop(guard);
    Ok(redact_sensitive(&result?))
}

async fn build_device_diagnostics(state: &AppState, device_id: &str) -> anyhow::Result<Value> {
    let status = require_compatible_project_firmware(state, device_id)
        .await
        .context("collect diagnostics status")?;
    let ports = usb_jsonl_request(state, device_id, "ports.get", None)
        .await
        .context("collect diagnostics ports")?;
    let wifi = usb_jsonl_request(state, device_id, "wifi.get", None)
        .await
        .context("collect diagnostics wifi")?;
    let (device, session) = {
        let inner = state.inner.lock().await;
        let device = inner
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow!("device not found"))?
            .clone();
        let session = json!({
            "logs": tail_items(&device.session.logs, 50),
            "traces": tail_items(&device.session.traces, 100),
        });
        (device, session)
    };

    Ok(json!({
        "device": device,
        "status": status,
        "ports": ports,
        "wifi": wifi,
        "session": session,
        "source": {
            "kind": "local_usb",
            "daemon": state.base_url,
            "generated_at_unix_ms": now_unix_millis(),
        }
    }))
}

pub async fn ipc_call(endpoint: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let request = IpcRequest {
        id: next_id(),
        method: method.to_string(),
        params,
    };
    #[cfg(unix)]
    {
        let stream = tokio::net::UnixStream::connect(endpoint)
            .await
            .with_context(|| format!("connect IPC socket {endpoint}"))?;
        send_ipc_request(stream, request).await
    }
    #[cfg(windows)]
    {
        let stream = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(endpoint)
            .with_context(|| format!("connect IPC pipe {endpoint}"))?;
        send_ipc_request(stream, request).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (endpoint, request);
        Err(anyhow!("isohub IPC is unsupported on this platform"))
    }
}

async fn send_ipc_request<S>(mut stream: S, request: IpcRequest) -> anyhow::Result<Value>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    stream.write_all(&encoded).await?;
    stream.flush().await?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    if line.trim().is_empty() {
        return Err(anyhow!("IPC daemon closed the connection without a response"));
    }
    let response: IpcResponse = serde_json::from_str(line.trim()).context("decode IPC response")?;
    if response.ok {
        Ok(response.result.unwrap_or_else(|| json!({})))
    } else {
        Err(anyhow!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "IPC request failed".to_string())
        ))
    }
}
