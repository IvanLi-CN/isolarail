pub async fn serve_http_bridge(config: DevdConfig) -> anyhow::Result<()> {
    if !config.bind.ip().is_loopback() {
        return Err(anyhow!(
            "isohub-devd web refuses non-loopback binds because /api/v1/bootstrap returns a local bearer token"
        ));
    }
    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("bind {}", config.bind))?;
    let port = listener.local_addr()?.port();
    let state = AppState::new(format!("http://127.0.0.1:{port}"));
    let mdns_advertiser = match crate::advertise_web_mdns(config.mdns_name.as_str(), port) {
        Ok(advertiser) => {
            tracing::info!(
                "isohub-devd web mDNS published service={} port={}",
                crate::WEB_MDNS_SERVICE_TYPE,
                port
            );
            Some(advertiser)
        }
        Err(err) => {
            tracing::warn!("isohub-devd web mDNS publish failed: {}", err);
            None
        }
    };

    let router = router(state, config.web_root, config.allow_dev_cors);
    tracing::info!("isohub-devd web listening on http://127.0.0.1:{port}");
    let _mdns_advertiser = mdns_advertiser;
    axum::serve(listener, router).await?;
    Ok(())
}

fn router(state: AppState, web_root: Option<PathBuf>, allow_dev_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/api/v1/bootstrap", get(bootstrap))
        .route("/api/v1/health", get(health))
        .route("/api/v1/devices", get(list_devices))
        .route("/api/v1/devices/scan", post(scan_devices))
        .route("/api/v1/devices/{id}/status", get(device_status))
        .route("/api/v1/devices/{id}/session", get(device_session))
        .route(
            "/api/v1/devices/{id}/wifi",
            get(wifi_get).post(wifi_set).delete(wifi_clear),
        )
        .route("/api/v1/devices/{id}/ports", get(device_ports))
        .route(
            "/api/v1/devices/{id}/ports/{port_id}/power",
            post(port_power),
        )
        .route(
            "/api/v1/devices/{id}/ports/{port_id}/replug",
            post(port_replug),
        )
        .route("/api/v1/devices/{id}/flash", post(device_flash))
        .route(
            "/api/v1/devices/{id}/flash-upload",
            post(device_flash_upload),
        )
        .route("/api/v1/devices/{id}/reset", post(device_reset))
        .route(
            "/api/v1/devices/{id}/diag-snapshot",
            get(device_diag_snapshot),
        )
        .route("/api/v1/devices/{id}/diagnostics", get(device_diagnostics))
        .route("/api/v1/serial/lease", post(create_lease))
        .route(
            "/api/v1/serial/lease/{lease_id}",
            post(heartbeat_lease).delete(release_lease),
        )
        .route(
            "/api/v1/storage/devices",
            get(storage_list).post(storage_save),
        )
        .route("/api/v1/storage/devices/{id}", delete(storage_delete))
        .route(
            "/api/v1/storage/settings",
            get(storage_settings_get).put(storage_settings_put),
        )
        .route(
            "/api/v1/storage/migrate/localstorage",
            post(storage_migrate_localstorage),
        )
        .route("/api/v1/storage/export", get(storage_export))
        .route("/api/v1/storage/reset", post(storage_reset))
        .route("/api/v1/storage/import", post(storage_import))
        .route("/api/v1/firmware/catalog/validate", post(validate_catalog))
        .with_state(state)
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024));

    if let Some(web_root) = web_root {
        router = router.fallback_service(ServeDir::new(web_root));
    }
    if allow_dev_cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _| {
                    is_loopback_origin(origin)
                }))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        );
    }
    router
}

async fn bootstrap(State(state): State<AppState>) -> Json<Value> {
    Json(json!(BootstrapResponse {
        token: state.token,
        agent_base_url: state.base_url,
        app: BootstrapApp {
            name: "isohub-devd",
            version: release_version(),
            mode: "web",
        },
    }))
}

async fn health(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    Json(json!({"ok": true})).into_response()
}

async fn list_devices(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    cleanup_expired_leases(&state).await;
    let devices = state
        .inner
        .lock()
        .await
        .devices
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Json(json!({"devices": devices})).into_response()
}

async fn scan_devices(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let ports = match list_serial_ports() {
        Ok(ports) => ports,
        Err(err) => return internal_error(&format!("serial enumeration failed: {err}")),
    };
    let mut inner = state.inner.lock().await;
    reconcile_scanned_usb_devices(&mut inner, ports);
    let devices = inner.devices.values().cloned().collect::<Vec<_>>();
    Json(json!({"devices": devices})).into_response()
}

fn reconcile_scanned_usb_devices(inner: &mut DevdState, ports: Vec<UsbTarget>) {
    let mut scanned_ids = HashSet::new();
    for port in ports {
        let id = stable_usb_device_id(&port.port_path);
        scanned_ids.insert(id.clone());
        inner
            .devices
            .entry(id.clone())
            .and_modify(|device| {
                device.display_name = port.label.clone();
                device.connection = "available".to_string();
                device.usb = Some(port.clone());
            })
            .or_insert(DeviceRecord {
                id,
                display_name: port.label.clone(),
                connection: "available".to_string(),
                usb: Some(port),
                http: None,
                identity: None,
                firmware_info: None,
                session: DeviceSession::default(),
            });
    }
    inner.devices.retain(|id, device| {
        if device.usb.is_some() && !scanned_ids.contains(id) {
            device.usb = None;
            if device.http.is_some() {
                device.connection = "unavailable".to_string();
                true
            } else {
                false
            }
        } else {
            true
        }
    });
}

async fn device_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match require_compatible_project_firmware(&state, &id).await {
        Ok(value) => {
            if let Err(err) = cache_project_firmware_info(&state, &id, &value).await {
                tracing::debug!(device_id = %id, "could not cache project firmware info: {err}");
            }
            Json(redact_sensitive(&value)).into_response()
        }
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeviceQuery>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let tail = query.tail.unwrap_or(200).min(MAX_SESSION_ITEMS);
    let inner = state.inner.lock().await;
    let Some(device) = inner.devices.get(&id) else {
        return not_found("device not found");
    };
    if let Some(lease_id) = query.lease_id.as_deref()
        && !inner.leases.contains_key(lease_id)
    {
        return unauthorized("lease not found or expired");
    }
    Json(json!({
        "logs": tail_items(&device.session.logs, tail),
        "traces": tail_items(&device.session.traces, tail),
    }))
    .into_response()
}

async fn wifi_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(&state, &id, "wifi.get", None).await {
        Ok(value) => {
            if let Err(err) = update_http_profile_from_usb_wifi(&state, &id, &value).await {
                tracing::warn!(
                    device_id = %id,
                    "could not update HTTP profile from Wi-Fi status: {err}"
                );
            }
            Json(redact_sensitive(&value)).into_response()
        }
        Err(err) => error_from_anyhow(err),
    }
}

async fn wifi_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<WifiRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(
        &state,
        &id,
        "wifi.set",
        Some(json!({"ssid": req.ssid, "psk": req.psk})),
    )
    .await
    {
        Ok(_) => match verify_wifi_after_set_timeout(&state, &id, &req.ssid).await {
            Ok(value) => {
                if let Err(err) = update_http_profile_from_usb_wifi(&state, &id, &value).await {
                    tracing::warn!(
                        device_id = %id,
                        "could not update HTTP profile from Wi-Fi status: {err}"
                    );
                }
                Json(redact_sensitive(&value)).into_response()
            }
            Err(err) => error_from_anyhow(err),
        },
        Err(err) => error_from_anyhow(err),
    }
}

async fn wifi_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(&state, &id, "wifi.clear", None).await {
        Ok(_) => match verify_wifi_after_clear_timeout(&state, &id).await {
            Ok(value) => {
                if let Err(err) = delete_http_profile_for_usb_device(&state, &id).await {
                    tracing::warn!(
                        device_id = %id,
                        "could not delete HTTP profile after Wi-Fi clear: {err}"
                    );
                }
                Json(redact_sensitive(&value)).into_response()
            }
            Err(err) => error_from_anyhow(err),
        },
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_ports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware_fast(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(&state, &id, "ports.get", None).await {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_diag_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware_fast(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(&state, &id, "hardware.snapshot", None).await {
        Ok(value) => {
            let snapshot = value.get("result").cloned().unwrap_or(value);
            Json(redact_sensitive(&snapshot)).into_response()
        }
        Err(err) => error_from_anyhow(err),
    }
}

async fn port_power(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, port_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware_fast(&state, &id).await {
        return error_from_anyhow(err);
    }
    let enabled = match query.get("enabled").map(String::as_str) {
        Some("1" | "true") => true,
        Some("0" | "false") => false,
        _ => return bad_request("enabled query must be one of 1, 0, true, false"),
    };
    match usb_jsonl_request(
        &state,
        &id,
        "port.power_set",
        Some(json!({"port": port_id, "enabled": enabled})),
    )
    .await
    {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn port_replug(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, port_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware_fast(&state, &id).await {
        return error_from_anyhow(err);
    }
    match usb_jsonl_request(&state, &id, "port.replug", Some(json!({"port": port_id}))).await {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_flash(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<FlashRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(response) = require_lease(&state, &id, req.lease_id.as_deref()).await {
        return *response;
    }
    let result = run_flash_request(&state, &id, req).await;
    match result {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_flash_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<FirmwareUploadFlashRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    if let Err(response) = require_lease(&state, &id, Some(&req.lease_id)).await {
        return *response;
    }
    let result = run_uploaded_flash_request(&state, &id, req).await;
    match result {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let lease_id = body.get("lease_id").and_then(Value::as_str);
    if let Err(response) = require_lease(&state, &id, lease_id).await {
        return *response;
    }
    if let Err(err) = require_compatible_project_firmware(&state, &id).await {
        return error_from_anyhow(err);
    }
    let port_path = match device_usb_port_path(&state, &id).await {
        Ok(port_path) => port_path,
        Err(err) => return error_from_anyhow(err),
    };
    {
        let mut inner = state.inner.lock().await;
        if inner.exclusive_ports.contains_key(&port_path) {
            return conflict("device busy");
        }
        inner
            .exclusive_ports
            .insert(port_path.clone(), "reset".to_string());
    }
    let guard = ExclusiveGuard {
        state: state.clone(),
        port_path,
    };
    let result = usb_jsonl_request_with_exclusive(&state, &id, "reboot", None, Some("reset")).await;
    drop(guard);
    match result {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn device_diagnostics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match build_device_diagnostics(&state, &id).await {
        Ok(value) => Json(redact_sensitive(&value)).into_response(),
        Err(err) => error_from_anyhow(err),
    }
}

async fn create_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LeaseRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    cleanup_expired_leases(&state).await;
    let port_path = {
        let inner = state.inner.lock().await;
        let Some(device) = inner.devices.get(&req.device_id) else {
            return not_found("device not found");
        };
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
    Json(json!(LeaseResponse {
        lease_id,
        device_id: req.device_id,
        heartbeat_interval_ms: LEASE_HEARTBEAT_INTERVAL_MS,
        lease_ttl_ms: LEASE_TTL_MS,
    }))
    .into_response()
}

async fn heartbeat_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let mut inner = state.inner.lock().await;
    let Some(lease) = inner.leases.get_mut(&lease_id) else {
        return not_found("lease not found");
    };
    lease.expires_at = Instant::now() + Duration::from_millis(LEASE_TTL_MS);
    Json(json!({
        "lease_id": lease.lease_id,
        "device_id": lease.device_id,
        "port_path": lease.port_path,
        "lease_ttl_ms": LEASE_TTL_MS,
    }))
    .into_response()
}

async fn release_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let removed = state.inner.lock().await.leases.remove(&lease_id).is_some();
    Json(json!({"ok": true, "released": removed})).into_response()
}

async fn storage_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match read_hardware_registry() {
        Ok(registry) => Json(json!({
            "devices": web_storage_devices(&registry),
            "profiles": registry.devices,
        }))
        .into_response(),
        Err(err) => internal_error(&format!("read storage failed: {err}")),
    }
}

async fn storage_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let input = match parse_storage_save_input(input) {
        Ok(input) => input,
        Err(err) => return bad_request(&err.to_string()),
    };
    match save_hardware_profiles(input) {
        Ok(devices) => {
            let web_device = read_hardware_registry()
                .ok()
                .and_then(|registry| {
                    devices
                        .first()
                        .and_then(|device| web_storage_device_for_profile(&registry, device))
                })
                .unwrap_or_else(|| web_storage_group_device(&devices.iter().collect::<Vec<_>>()));
            Json(json!({
                "device": web_device,
                "profiles": devices,
            }))
            .into_response()
        }
        Err(err) => bad_request(&format!("save storage failed: {err}")),
    }
}

async fn storage_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match delete_hardware(&id) {
        Ok(removed) => Json(json!({"removed": removed})).into_response(),
        Err(err) => bad_request(&format!("delete storage failed: {err}")),
    }
}

async fn storage_settings_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match read_storage_settings() {
        Ok(settings) => Json(json!({"settings": settings})).into_response(),
        Err(err) => internal_error(&format!("read settings failed: {err}")),
    }
}

async fn storage_settings_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StorageSettingsRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match write_storage_settings(&req.settings) {
        Ok(()) => Json(json!({"settings": req.settings})).into_response(),
        Err(err) => bad_request(&format!("write settings failed: {err}")),
    }
}

async fn storage_migrate_localstorage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    match migrate_localstorage_payload(input) {
        Ok((devices, settings_written)) => Json(json!({
            "migrated": devices > 0 || settings_written,
            "imported": {"devices": devices, "settings": settings_written},
        }))
        .into_response(),
        Err(err) => bad_request(&format!("migration failed: {err}")),
    }
}

async fn storage_export(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let registry = match read_hardware_registry() {
        Ok(registry) => registry,
        Err(err) => return internal_error(&format!("read storage failed: {err}")),
    };
    let settings = match read_storage_settings() {
        Ok(settings) => settings,
        Err(err) => return internal_error(&format!("read settings failed: {err}")),
    };
    Json(json!({
        "schema_version": STORAGE_SCHEMA_VERSION,
        "devices": web_storage_devices(&registry),
        "profiles": registry.devices,
        "settings": settings,
        "meta": {},
    }))
    .into_response()
}

async fn storage_reset(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let registry = HardwareRegistry::default();
    if let Err(err) = write_hardware_registry(&registry) {
        return bad_request(&format!("reset storage failed: {err}"));
    }
    if let Err(err) = write_storage_settings(&default_storage_settings()) {
        return bad_request(&format!("reset settings failed: {err}"));
    }
    Json(json!({"ok": true})).into_response()
}

fn parse_storage_save_input(value: Value) -> anyhow::Result<Vec<SavedHardwareInput>> {
    if let Some(device) = value.get("device").and_then(Value::as_object) {
        let name = device
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("device.name is required"))?
            .to_string();
        let base_url = device
            .get("baseUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("device.baseUrl is required"))?
            .to_string();
        let id = device
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| stable_http_device_id(&base_url));
        let identity = default_hostname_short_id(&base_url).map(|device_id| DeviceIdentity {
            device_id: Some(device_id),
            mac: None,
        });
        let transports = device.get("transports").and_then(Value::as_object);
        let http_base_url = transports
            .and_then(|transports| transports.get("httpBaseUrl"))
            .and_then(Value::as_str)
            .unwrap_or(&base_url);
        let mut inputs = vec![SavedHardwareInput {
            id: id.clone(),
            name: name.clone(),
            transport: HardwareTransport::Http {
                base_url: http_base_url.to_string(),
            },
            identity: identity.clone(),
        }];
        if let Some(local_usb_device_id) = transports
            .and_then(|transports| transports.get("localUsbDeviceId"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            inputs.push(SavedHardwareInput {
                id: format!("{id}--usb"),
                name: name.clone(),
                transport: HardwareTransport::Usb {
                    device_id: local_usb_device_id.to_string(),
                    devd_url: None,
                },
                identity: identity.clone(),
            });
        }
        if let Some(web_serial_label) = transports
            .and_then(|transports| transports.get("webSerialLabel"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            inputs.push(SavedHardwareInput {
                id: format!("{id}--webserial"),
                name: name.clone(),
                transport: HardwareTransport::WebSerial {
                    label: Some(web_serial_label.to_string()),
                },
                identity: identity.clone(),
            });
        }
        return Ok(inputs);
    }
    #[derive(Deserialize)]
    struct Wire {
        id: String,
        name: String,
        transport: HardwareTransport,
    }
    let wire: Wire = serde_json::from_value(value)?;
    Ok(vec![SavedHardwareInput {
        id: wire.id,
        name: wire.name,
        transport: wire.transport,
        identity: None,
    }])
}

fn web_storage_devices(registry: &HardwareRegistry) -> Vec<Value> {
    let mut groups: Vec<(String, Vec<&DeviceProfile>)> = Vec::new();
    for profile in &registry.devices {
        let key = profile_group_key(profile);
        if let Some((_, profiles)) = groups.iter_mut().find(|(existing, _)| existing == &key) {
            profiles.push(profile);
        } else {
            groups.push((key, vec![profile]));
        }
    }
    groups
        .into_iter()
        .map(|(_, profiles)| web_storage_group_device(&profiles))
        .collect()
}

fn web_storage_device_for_profile(
    registry: &HardwareRegistry,
    profile: &DeviceProfile,
) -> Option<Value> {
    let key = profile_group_key(profile);
    let profiles = registry
        .devices
        .iter()
        .filter(|candidate| profile_group_key(candidate) == key)
        .collect::<Vec<_>>();
    (!profiles.is_empty()).then(|| web_storage_group_device(&profiles))
}

fn web_storage_group_device(profiles: &[&DeviceProfile]) -> Value {
    let primary = profiles
        .iter()
        .find(|profile| matches!(profile.transport, HardwareTransport::Usb { .. }))
        .or_else(|| profiles.first())
        .expect("web storage group must contain at least one profile");

    let mut http_base_url = None;
    let mut local_usb_device_id = None;
    let mut web_serial_label = None;
    let mut last_seen_at = primary.last_seen_at;
    for profile in profiles {
        last_seen_at = last_seen_at.max(profile.last_seen_at);
        match &profile.transport {
            HardwareTransport::Http { base_url } => {
                http_base_url = Some(base_url.clone());
            }
            HardwareTransport::Usb { device_id, .. } => {
                local_usb_device_id = Some(device_id.clone());
            }
            HardwareTransport::WebSerial { label } => {
                web_serial_label = label.clone().or_else(|| Some(profile.id.clone()));
            }
        }
    }

    let base_url = http_base_url
        .clone()
        .unwrap_or_else(|| match &primary.transport {
            HardwareTransport::Http { base_url } => base_url.clone(),
            HardwareTransport::Usb { device_id, .. } => format!("isohub-devd://{device_id}"),
            HardwareTransport::WebSerial { label } => {
                format!("webserial://{}", label.as_deref().unwrap_or(&primary.id))
            }
        });

    let mut transports = serde_json::Map::new();
    if let Some(value) = http_base_url {
        transports.insert("httpBaseUrl".to_string(), json!(value));
    }
    if let Some(value) = local_usb_device_id {
        transports.insert("localUsbDeviceId".to_string(), json!(value));
    }
    if let Some(value) = web_serial_label {
        transports.insert("webSerialLabel".to_string(), json!(value));
    }

    json!({
        "id": web_storage_public_id(profiles, primary),
        "name": primary.name,
        "baseUrl": base_url,
        "lastSeenAt": last_seen_at.map(|ts| ts.to_string()),
        "transports": Value::Object(transports),
    })
}

fn web_storage_public_id(profiles: &[&DeviceProfile], primary: &DeviceProfile) -> String {
    profiles
        .iter()
        .find(|profile| !is_transport_variant_profile_id(&profile.id))
        .map(|profile| profile.id.clone())
        .or_else(|| {
            profiles.iter().find_map(|profile| {
                profile
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.device_id.as_deref())
                    .map(str::to_string)
            })
        })
        .unwrap_or_else(|| primary.id.clone())
}

fn is_transport_variant_profile_id(id: &str) -> bool {
    id.ends_with("--http") || id.ends_with("--usb") || id.ends_with("--webserial")
}

fn profile_group_key(profile: &DeviceProfile) -> String {
    profile_identity_key(profile).unwrap_or_else(|| {
        let id = profile.id.trim().to_ascii_lowercase();
        if is_short_device_id(&id) {
            format!("device:{id}")
        } else {
            format!("id:{id}")
        }
    })
}

fn profile_identity_key(profile: &DeviceProfile) -> Option<String> {
    profile
        .identity
        .as_ref()
        .and_then(device_identity_key)
        .or_else(|| match &profile.transport {
            HardwareTransport::Http { base_url } => {
                default_hostname_short_id(base_url).map(|id| format!("device:{id}"))
            }
            _ => None,
        })
}

fn device_identity_key(identity: &DeviceIdentity) -> Option<String> {
    identity
        .device_id
        .as_deref()
        .map(normalize_device_id)
        .filter(|id| !id.is_empty())
        .or_else(|| identity.mac.as_deref().and_then(mac_short_id))
        .map(|id| format!("device:{id}"))
}

fn normalize_device_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn mac_short_id(value: &str) -> Option<String> {
    let hex = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    if hex.len() < 6 {
        return None;
    }
    let short_id = &hex[hex.len() - 6..];
    is_short_device_id(short_id).then(|| short_id.to_string())
}

fn default_hostname_short_id(base_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(base_url).ok()?;
    let host = url.host_str()?.trim_end_matches('.').to_ascii_lowercase();
    let host = host.strip_suffix(".local").unwrap_or(&host);
    let short_id = host.strip_prefix("isohub-")?;
    is_short_device_id(short_id).then(|| short_id.to_string())
}

fn is_short_device_id(value: &str) -> bool {
    value.len() == 6 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn hardware_transport_from_storage_url(base_url: &str) -> HardwareTransport {
    if let Some(device_id) = base_url.strip_prefix("isohub-devd://") {
        HardwareTransport::Usb {
            device_id: device_id.to_string(),
            devd_url: None,
        }
    } else if let Some(label) = base_url.strip_prefix("webserial://") {
        HardwareTransport::WebSerial {
            label: Some(label.to_string()),
        }
    } else {
        HardwareTransport::Http {
            base_url: base_url.to_string(),
        }
    }
}

fn parse_web_storage_device(value: &Value) -> anyhow::Result<DeviceProfile> {
    let device = value
        .as_object()
        .ok_or_else(|| anyhow!("device must be an object"))?;
    let id = device
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("device.id is required"))?
        .to_string();
    let name = device
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("device.name is required"))?
        .to_string();
    let base_url = device
        .get("baseUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("device.baseUrl is required"))?;
    Ok(DeviceProfile {
        id,
        name,
        transport: hardware_transport_from_storage_url(base_url),
        identity: None,
        last_seen_at: Some(now_unix_seconds()),
    })
}

fn parse_import_profiles(req: &StorageImportRequest) -> anyhow::Result<Vec<DeviceProfile>> {
    if !req.profiles.is_empty() {
        return Ok(req.profiles.clone());
    }
    req.devices
        .iter()
        .cloned()
        .map(|device| {
            if device.get("transport").is_some() {
                serde_json::from_value(device).context("parse device profile")
            } else {
                parse_web_storage_device(&device)
            }
        })
        .collect()
}

fn migrate_localstorage_payload(value: Value) -> anyhow::Result<(usize, bool)> {
    let mut imported_devices = 0;
    if let Some(devices) = value.get("devices").and_then(Value::as_array) {
        let mut registry = read_hardware_registry()?;
        for device in devices {
            upsert_profile(&mut registry, parse_web_storage_device(device)?);
            imported_devices += 1;
        }
        write_hardware_registry(&registry)?;
    }

    let mut settings_written = false;
    if let Some(theme) = value
        .get("settings")
        .and_then(|settings| settings.get("theme"))
        .and_then(Value::as_str)
    {
        write_storage_settings(&StorageSettings {
            theme: theme.to_string(),
        })?;
        settings_written = true;
    }
    Ok((imported_devices, settings_written))
}

async fn storage_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StorageImportRequest>,
) -> Response {
    if let Err(response) = require_auth(&headers, &state) {
        return *response;
    }
    let profiles = match parse_import_profiles(&req) {
        Ok(profiles) => profiles,
        Err(err) => return bad_request(&format!("import failed: {err}")),
    };
    let settings = req.settings;
    match import_profiles(profiles) {
        Ok(count) => {
            let settings_written = if let Some(settings) = settings {
                if let Err(err) = write_storage_settings(&settings) {
                    return bad_request(&format!("import settings failed: {err}"));
                }
                true
            } else {
                false
            };
            Json(json!({"imported": {"devices": count, "settings": settings_written}}))
                .into_response()
        }
        Err(err) => bad_request(&format!("import failed: {err}")),
    }
}
